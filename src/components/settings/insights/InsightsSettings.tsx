import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { ask } from "@tauri-apps/plugin-dialog";
import { Loader2 } from "lucide-react";
import { commands, type UsageSummary } from "@/bindings";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";

/** Minutes, rounded, from milliseconds. */
const minutes = (ms: number) => Math.round(ms / 60000);

function formatDuration(
  ms: number,
  t: ReturnType<typeof useTranslation>["t"],
): string {
  const total = minutes(ms);
  const hours = Math.floor(total / 60);
  const rest = total % 60;

  return hours > 0
    ? t("settings.insights.hoursMinutes", { hours, minutes: rest })
    : t("settings.insights.minutes", { minutes: rest });
}

/**
 * What Murmel knows about how it is used — read from `usage_stats`, which holds
 * numbers only and never leaves this machine.
 *
 * Every figure here is a by-product of dictating; nothing is measured for its
 * own sake. See Murmel_Northstar.md §6.
 */
export const InsightsSettings: React.FC = () => {
  const { t, i18n } = useTranslation();
  const [summary, setSummary] = useState<UsageSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const result = await commands.getUsageSummary();
      if (result.status === "ok") setSummary(result.data);
    } catch (error) {
      console.error("Failed to load usage summary:", error);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const exportStats = async (format: "json" | "csv") => {
    setBusy(true);
    try {
      const result = await commands.exportUsageStats();
      if (result.status !== "ok") return;

      const rows = result.data;
      const path = await save({
        defaultPath: `murmel-statistik.${format}`,
        filters: [{ name: format.toUpperCase(), extensions: [format] }],
      });
      if (!path) return;

      if (format === "json") {
        await writeTextFile(path, JSON.stringify(rows, null, 2));
      } else {
        const columns = Object.keys(rows[0] ?? { timestamp: 0 });
        const escape = (value: unknown) => {
          if (value === null || value === undefined) return "";
          const text = String(value);
          return /[",\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
        };
        const lines = [
          columns.join(","),
          ...rows.map((row) =>
            columns
              .map((column) => escape((row as Record<string, unknown>)[column]))
              .join(","),
          ),
        ];
        await writeTextFile(path, lines.join("\n"));
      }
    } catch (error) {
      console.error("Failed to export statistics:", error);
    } finally {
      setBusy(false);
    }
  };

  const clearStats = async () => {
    const confirmed = await ask(t("settings.insights.clearConfirm"), {
      title: t("settings.insights.clearTitle"),
      kind: "warning",
    });
    if (!confirmed) return;

    setBusy(true);
    try {
      await commands.clearUsageStats();
      await load();
    } finally {
      setBusy(false);
    }
  };

  if (loading) {
    return (
      <div className="max-w-3xl w-full mx-auto flex items-center justify-center py-16">
        <Loader2 className="w-6 h-6 animate-spin text-text/40" />
      </div>
    );
  }

  const empty = !summary || summary.dictations === 0;

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <div>
        <h1 className="text-xl font-semibold mb-2">
          {t("settings.insights.title")}
        </h1>
        <p className="text-sm text-text/60">
          {t("settings.insights.description")}
        </p>
      </div>

      {empty ? (
        <div className="text-center py-12 text-text/50">
          {t("settings.insights.empty")}
        </div>
      ) : (
        <>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            <Stat
              label={t("settings.insights.dictations")}
              value={summary.dictations.toLocaleString(i18n.language)}
            />
            <Stat
              label={t("settings.insights.words")}
              value={summary.words.toLocaleString(i18n.language)}
            />
            <Stat
              label={t("settings.insights.spoken")}
              value={formatDuration(summary.duration_ms, t)}
            />
            <Stat
              label={t("settings.insights.saved")}
              value={formatDuration(summary.saved_ms, t)}
              emphasis
            />
          </div>

          <DailyChart summary={summary} />

          <SettingsGroup title={t("settings.insights.details.title")}>
            <SettingContainer
              title={t("settings.insights.details.pace")}
              description={t("settings.insights.details.paceDescription")}
              grouped
            >
              <span className="text-sm font-mono">
                {summary.duration_ms > 0
                  ? t("settings.insights.wpm", {
                      wpm: Math.round(
                        summary.words / (summary.duration_ms / 60000),
                      ),
                    })
                  : "—"}
              </span>
            </SettingContainer>

            {summary.busiest_hour !== null && (
              <SettingContainer
                title={t("settings.insights.details.busiestHour")}
                description={t(
                  "settings.insights.details.busiestHourDescription",
                )}
                grouped
              >
                <span className="text-sm font-mono">
                  {t("settings.insights.hourRange", {
                    from: String(summary.busiest_hour).padStart(2, "0"),
                    to: String((summary.busiest_hour + 1) % 24).padStart(
                      2,
                      "0",
                    ),
                  })}
                </span>
              </SettingContainer>
            )}

            <SettingContainer
              title={t("settings.insights.details.refined")}
              description={t("settings.insights.details.refinedDescription")}
              grouped
            >
              <span className="text-sm font-mono">
                {t("settings.insights.ofDictations", {
                  count: summary.refined,
                  total: summary.dictations,
                })}
                {summary.refinement_failures > 0 && (
                  <span className="ms-2 text-warning">
                    {t("settings.insights.failures", {
                      count: summary.refinement_failures,
                    })}
                  </span>
                )}
              </span>
            </SettingContainer>

            {summary.models.map((model) => (
              <SettingContainer
                key={model.model}
                title={model.model}
                description={t("settings.insights.details.modelDescription")}
                grouped
              >
                <span className="text-sm font-mono">
                  {t("settings.insights.modelSummary", {
                    count: model.dictations,
                    seconds: (model.average_processing_ms / 1000).toFixed(1),
                  })}
                </span>
              </SettingContainer>
            ))}
          </SettingsGroup>
        </>
      )}

      <SettingsGroup title={t("settings.insights.data.title")}>
        <SettingContainer
          title={t("settings.insights.data.export")}
          description={t("settings.insights.data.exportDescription")}
          grouped
        >
          <div className="flex items-center gap-2">
            {/* File formats, not prose — the same in every language. */}
            <Button
              variant="secondary"
              size="sm"
              onClick={() => exportStats("json")}
              disabled={busy || empty}
            >
              JSON
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => exportStats("csv")}
              disabled={busy || empty}
            >
              CSV
            </Button>
          </div>
        </SettingContainer>

        <SettingContainer
          title={t("settings.insights.data.clear")}
          description={t("settings.insights.data.clearDescription")}
          grouped
        >
          <Button
            variant="danger"
            size="sm"
            onClick={clearStats}
            disabled={busy || empty}
          >
            {t("settings.insights.data.clearButton")}
          </Button>
        </SettingContainer>
      </SettingsGroup>
    </div>
  );
};

const Stat: React.FC<{
  label: string;
  value: string;
  emphasis?: boolean;
}> = ({ label, value, emphasis }) => (
  <div
    className={`rounded-xl border-2 px-4 py-3 ${
      emphasis
        ? "border-logo-primary/40 bg-logo-primary/10"
        : "border-mid-gray/20"
    }`}
  >
    <div className="text-xl font-semibold tabular-nums">{value}</div>
    <div className="text-xs text-text/60 mt-0.5">{label}</div>
  </div>
);

/**
 * Words per day as plain bars.
 *
 * No chart library: a row of divs scaled against the busiest day says the same
 * thing, works in both themes without configuration, and adds nothing to the
 * bundle.
 */
const DailyChart: React.FC<{ summary: UsageSummary }> = ({ summary }) => {
  const { t } = useTranslation();
  const peak = Math.max(...summary.per_day.map((day) => day.words), 1);

  if (summary.per_day.length === 0) return null;

  return (
    <div className="space-y-2">
      <h2 className="text-sm font-medium text-text/60">
        {t("settings.insights.perDay")}
      </h2>
      <div className="flex items-end gap-1 h-28 rounded-xl border-2 border-mid-gray/20 px-3 py-2">
        {summary.per_day.map((day) => (
          <div
            key={day.day}
            title={t("settings.insights.dayTooltip", {
              day: day.day,
              words: day.words,
            })}
            className="flex-1 min-w-[3px] rounded-sm bg-logo-primary/60 hover:bg-logo-primary transition-colors"
            style={{
              // A day with a single word still deserves a visible sliver.
              height: `${Math.max((day.words / peak) * 100, 3)}%`,
            }}
          />
        ))}
      </div>
    </div>
  );
};
