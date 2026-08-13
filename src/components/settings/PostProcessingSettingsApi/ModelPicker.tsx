import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Cloud, HardDrive, RefreshCw, Search } from "lucide-react";
import { commands, type LocalModel, type RemoteModel } from "@/bindings";
import { OLLAMA_PROVIDER_ID, OPENROUTER_PROVIDER_ID } from "./types";

type Source = "local" | "remote";

interface ModelPickerProps {
  activeProviderId: string;
  activeModel: string;
  /** Sets provider and model together — picking a model is one decision. */
  onPick: (providerId: string, model: string) => void;
}

/** Words the average dictation is assumed to have until history says otherwise. */
const ASSUMED_WORDS = 60;

/**
 * Rough token count for a German dictation plus the instruction wrapped around
 * it.
 *
 * Deliberately a rule of thumb, not a tokeniser: every model tokenises
 * differently, and the number exists to tell "fractions of a cent" apart from
 * "several cents" — a second decimal place would suggest a precision that no
 * estimate here has.
 */
const TOKENS_PER_WORD = 1.6;
const PROMPT_OVERHEAD_TOKENS = 180;

function costPerDictation(model: RemoteModel, words: number): number {
  const inputTokens = words * TOKENS_PER_WORD + PROMPT_OVERHEAD_TOKENS;
  // Refinement returns roughly what it was given — it tidies, it does not write
  // an essay.
  const outputTokens = words * TOKENS_PER_WORD;

  return (
    inputTokens * model.prompt_price + outputTokens * model.completion_price
  );
}

function formatSize(bytes: number): string {
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

/**
 * The refinement model picker: one list per source, never both at once.
 *
 * Local and remote models answer different questions — "does it fit on this
 * machine" versus "what does it cost me" — so they get different columns and a
 * switch instead of a merged list with half-empty rows.
 */
export const ModelPicker: React.FC<ModelPickerProps> = ({
  activeProviderId,
  activeModel,
  onPick,
}) => {
  const { t } = useTranslation();
  const [source, setSource] = useState<Source>(
    activeProviderId === OPENROUTER_PROVIDER_ID ? "remote" : "local",
  );
  const [localModels, setLocalModels] = useState<LocalModel[]>([]);
  const [remoteModels, setRemoteModels] = useState<RemoteModel[]>([]);
  const [averageWords, setAverageWords] = useState<number | null>(null);
  const [query, setQuery] = useState("");
  const [freeOnly, setFreeOnly] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      if (source === "local") {
        const result = await commands.listLocalLlmModels();
        if (result.status === "ok") setLocalModels(result.data);
        else setError(String(result.error));
      } else {
        const [catalog, words] = await Promise.all([
          commands.listRemoteLlmModels(),
          commands.getAverageDictationWords(),
        ]);
        if (catalog.status === "ok") setRemoteModels(catalog.data);
        else setError(String(catalog.error));
        if (words.status === "ok") setAverageWords(words.data);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [source]);

  useEffect(() => {
    void load();
  }, [load]);

  const words = averageWords ?? ASSUMED_WORDS;

  const visibleLocal = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return localModels.filter(
      (model) => !needle || model.name.toLowerCase().includes(needle),
    );
  }, [localModels, query]);

  const visibleRemote = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return remoteModels.filter((model) => {
      if (freeOnly && !model.is_free) return false;
      if (!needle) return true;
      return (
        model.name.toLowerCase().includes(needle) ||
        model.id.toLowerCase().includes(needle)
      );
    });
  }, [remoteModels, query, freeOnly]);

  const isActive = (providerId: string, model: string) =>
    activeProviderId === providerId && activeModel === model;

  return (
    <div className="space-y-3">
      {/* Source switch — the one decision that changes everything below it. */}
      <div className="flex items-center gap-1 p-1 rounded-lg bg-mid-gray/10 w-fit">
        <SourceButton
          active={source === "local"}
          onClick={() => setSource("local")}
          icon={<HardDrive className="w-3.5 h-3.5" />}
          label={t("settings.postProcessing.models.local")}
        />
        <SourceButton
          active={source === "remote"}
          onClick={() => setSource("remote")}
          icon={<Cloud className="w-3.5 h-3.5" />}
          label={t("settings.postProcessing.models.remote")}
        />
      </div>

      <div className="flex items-center gap-2">
        <div className="relative flex-1">
          <Search className="absolute start-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text/40 pointer-events-none" />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t("settings.postProcessing.models.searchPlaceholder")}
            aria-label={t("settings.postProcessing.models.searchPlaceholder")}
            className="w-full ps-9 pe-3 py-2 text-sm rounded-md bg-mid-gray/10 border border-mid-gray/40 text-text placeholder:text-text/40 focus:outline-none focus:border-logo-primary/50"
          />
        </div>

        {source === "remote" && (
          <button
            type="button"
            onClick={() => setFreeOnly((on) => !on)}
            aria-pressed={freeOnly}
            title={t("settings.postProcessing.models.freeOnly")}
            className={`h-9 px-3 text-sm font-medium rounded-md transition-colors ${
              freeOnly
                ? "bg-logo-primary/20 text-logo-primary"
                : "bg-mid-gray/10 text-text/60 hover:bg-mid-gray/20"
            }`}
          >
            {t("settings.postProcessing.models.freeOnly")}
          </button>
        )}

        <button
          type="button"
          onClick={() => void load()}
          disabled={loading}
          title={t("settings.postProcessing.models.refresh")}
          aria-label={t("settings.postProcessing.models.refresh")}
          className="flex items-center justify-center w-9 h-9 rounded-md bg-mid-gray/10 text-text/60 hover:bg-mid-gray/20 transition-colors disabled:opacity-50"
        >
          <RefreshCw
            className={`w-3.5 h-3.5 ${loading ? "animate-spin" : ""}`}
          />
        </button>
      </div>

      {error && (
        <p className="text-sm text-error">
          {source === "local"
            ? t("settings.postProcessing.models.localUnavailable")
            : error}
        </p>
      )}

      {source === "remote" && !error && (
        <p className="text-xs text-text/50">
          {averageWords === null
            ? t("settings.postProcessing.models.costAssumed", {
                words: ASSUMED_WORDS,
              })
            : t("settings.postProcessing.models.costMeasured", {
                words: Math.round(averageWords),
              })}
        </p>
      )}

      <div className="border border-mid-gray/20 rounded-lg divide-y divide-mid-gray/15 overflow-hidden">
        {source === "local"
          ? visibleLocal.map((model) => (
              <ModelRow
                key={model.name}
                title={model.name}
                detail={[
                  formatSize(model.size_bytes),
                  model.parameter_size,
                  model.quantization,
                ]
                  .filter(Boolean)
                  .join(" · ")}
                active={isActive(OLLAMA_PROVIDER_ID, model.name)}
                onSelect={() => onPick(OLLAMA_PROVIDER_ID, model.name)}
                activeLabel={t("settings.postProcessing.models.active")}
              />
            ))
          : visibleRemote.map((model) => (
              <ModelRow
                key={model.id}
                title={model.name}
                detail={[
                  `${Math.round(model.context_length / 1000)}k`,
                  model.is_free
                    ? t("settings.postProcessing.models.free")
                    : t("settings.postProcessing.models.perDictation", {
                        cents: (costPerDictation(model, words) * 100).toFixed(
                          3,
                        ),
                      }),
                ].join(" · ")}
                active={isActive(OPENROUTER_PROVIDER_ID, model.id)}
                onSelect={() => onPick(OPENROUTER_PROVIDER_ID, model.id)}
                activeLabel={t("settings.postProcessing.models.active")}
              />
            ))}

        {!loading &&
          (source === "local" ? visibleLocal : visibleRemote).length === 0 &&
          !error && (
            <p className="px-4 py-6 text-center text-sm text-text/50">
              {t("settings.postProcessing.models.noMatches")}
            </p>
          )}
      </div>
    </div>
  );
};

const SourceButton: React.FC<{
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
}> = ({ active, onClick, icon, label }) => (
  <button
    type="button"
    onClick={onClick}
    aria-pressed={active}
    className={`flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-md transition-colors ${
      active
        ? "bg-background text-text shadow-sm"
        : "text-text/60 hover:text-text"
    }`}
  >
    {icon}
    {label}
  </button>
);

const ModelRow: React.FC<{
  title: string;
  detail: string;
  active: boolean;
  activeLabel: string;
  onSelect: () => void;
}> = ({ title, detail, active, activeLabel, onSelect }) => (
  <button
    type="button"
    onClick={onSelect}
    className={`w-full flex items-center justify-between gap-3 px-4 py-2.5 text-left transition-colors ${
      active ? "bg-logo-primary/10" : "hover:bg-mid-gray/10"
    }`}
  >
    <span className="min-w-0">
      <span className="block text-sm font-medium truncate">{title}</span>
      <span className="block text-xs text-text/50">{detail}</span>
    </span>
    {active && (
      <span className="flex items-center gap-1 text-xs font-medium text-logo-primary shrink-0">
        <Check className="w-3.5 h-3.5" />
        {activeLabel}
      </span>
    )}
  </button>
);
