import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Check,
  Cloud,
  Coins,
  Cpu,
  HardDrive,
  Loader2,
  RefreshCw,
  Search,
  Sparkles,
} from "lucide-react";
import { commands, type LocalModel, type RemoteModel } from "@/bindings";
import Badge from "../../ui/Badge";
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

/**
 * The refinement model picker: one list per source, never both at once.
 *
 * Local and remote models answer different questions — "does it fit on this
 * machine" versus "what does it cost me" — so they get different footers and a
 * switch instead of a merged list with half-empty rows. The card layout follows
 * the transcription models page so both model screens read the same way.
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

  /**
   * The chosen model, pulled out of the list into its own section.
   *
   * Otherwise it sits wherever the sort order puts it — with several hundred
   * entries that can be far down, and "which one am I actually using" becomes a
   * scrolling exercise. Deliberately shown even when the search would filter it
   * out: the question it answers is not "what did I search for".
   */
  const activeLocal = useMemo(
    () =>
      activeProviderId === OLLAMA_PROVIDER_ID
        ? (localModels.find((model) => model.name === activeModel) ?? null)
        : null,
    [localModels, activeProviderId, activeModel],
  );

  const activeRemote = useMemo(
    () =>
      activeProviderId === OPENROUTER_PROVIDER_ID
        ? (remoteModels.find((model) => model.id === activeModel) ?? null)
        : null,
    [remoteModels, activeProviderId, activeModel],
  );

  const isEmpty =
    !loading &&
    !error &&
    (source === "local" ? visibleLocal : visibleRemote).length === 0;

  const localCard = (model: LocalModel) => (
    <ModelCard
      key={model.name}
      title={model.name}
      subtitle={model.family ?? ""}
      active={
        model.name === activeModel && activeProviderId === OLLAMA_PROVIDER_ID
      }
      activeLabel={t("settings.postProcessing.api.models.active")}
      onSelect={() => onPick(OLLAMA_PROVIDER_ID, model.name)}
      facts={[
        {
          icon: <HardDrive className="w-3.5 h-3.5" />,
          text: `${(model.size_bytes / 1024 ** 3).toFixed(1)} GB`,
        },
        model.parameter_size
          ? {
              icon: <Cpu className="w-3.5 h-3.5" />,
              text: model.parameter_size,
            }
          : null,
        model.quantization
          ? {
              icon: <Sparkles className="w-3.5 h-3.5" />,
              text: model.quantization,
            }
          : null,
      ]}
    />
  );

  const remoteCard = (model: RemoteModel) => (
    <ModelCard
      key={model.id}
      title={model.name}
      subtitle={model.id}
      active={
        model.id === activeModel && activeProviderId === OPENROUTER_PROVIDER_ID
      }
      activeLabel={t("settings.postProcessing.api.models.active")}
      badge={
        model.is_free ? t("settings.postProcessing.api.models.free") : undefined
      }
      onSelect={() => onPick(OPENROUTER_PROVIDER_ID, model.id)}
      facts={[
        {
          icon: <Cpu className="w-3.5 h-3.5" />,
          text: t("settings.postProcessing.api.models.context", {
            thousands: Math.round(model.context_length / 1000),
          }),
        },
        model.is_free
          ? null
          : {
              icon: <Coins className="w-3.5 h-3.5" />,
              text: t("settings.postProcessing.api.models.perDictation", {
                cents: (costPerDictation(model, words) * 100).toFixed(3),
              }),
            },
      ]}
    />
  );

  const activeCard =
    source === "local"
      ? activeLocal && localCard(activeLocal)
      : activeRemote && remoteCard(activeRemote);

  return (
    <div className="w-full space-y-4">
      {/* Source switch — the one decision that changes everything below it. */}
      <div className="flex items-center gap-1 p-1 rounded-lg bg-mid-gray/10 w-fit">
        <SourceButton
          active={source === "local"}
          onClick={() => setSource("local")}
          icon={<HardDrive className="w-3.5 h-3.5" />}
          label={t("settings.postProcessing.api.models.local")}
        />
        <SourceButton
          active={source === "remote"}
          onClick={() => setSource("remote")}
          icon={<Cloud className="w-3.5 h-3.5" />}
          label={t("settings.postProcessing.api.models.remote")}
        />
      </div>

      <div className="relative">
        <Search className="absolute start-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text/40 pointer-events-none" />
        <input
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder={t(
            "settings.postProcessing.api.models.searchPlaceholder",
          )}
          aria-label={t("settings.postProcessing.api.models.searchPlaceholder")}
          className="w-full ps-9 pe-3 py-2 text-sm bg-mid-gray/10 border border-mid-gray/40 rounded-lg focus:outline-none focus:ring-1 focus:ring-logo-primary placeholder:text-text/40"
        />
      </div>

      {/* The chosen model gets its own section above the catalogue, so it is
          always the first thing seen and never buried in a long list. */}
      {activeCard && !loading && (
        <div className="space-y-2">
          <h3 className="text-sm font-medium text-text/60">
            {t("settings.postProcessing.api.models.activeSection")}
          </h3>
          {activeCard}
        </div>
      )}

      <div className="flex items-center justify-between gap-2">
        <h3 className="text-sm font-medium text-text/60">
          {source === "local"
            ? t("settings.postProcessing.api.models.installed")
            : t("settings.postProcessing.api.models.available")}
        </h3>

        <div className="flex items-center gap-2">
          {source === "remote" && (
            <button
              type="button"
              onClick={() => setFreeOnly((on) => !on)}
              aria-pressed={freeOnly}
              title={t("settings.postProcessing.api.models.freeOnly")}
              className={`h-8 px-3 text-sm font-medium rounded-lg transition-colors ${
                freeOnly
                  ? "bg-logo-primary/20 text-logo-primary hover:bg-logo-primary/30"
                  : "bg-mid-gray/10 text-text/60 hover:bg-mid-gray/20"
              }`}
            >
              {t("settings.postProcessing.api.models.freeOnly")}
            </button>
          )}
          <button
            type="button"
            onClick={() => void load()}
            disabled={loading}
            title={t("settings.postProcessing.api.models.refresh")}
            aria-label={t("settings.postProcessing.api.models.refresh")}
            className="flex items-center justify-center w-8 h-8 rounded-lg bg-mid-gray/10 text-text/60 hover:bg-mid-gray/20 transition-colors disabled:opacity-50"
          >
            <RefreshCw
              className={`w-3.5 h-3.5 ${loading ? "animate-spin" : ""}`}
            />
          </button>
        </div>
      </div>

      {error && (
        <p className="text-sm text-error">
          {source === "local"
            ? t("settings.postProcessing.api.models.localUnavailable")
            : error}
        </p>
      )}

      {source === "remote" && !error && (
        <p className="text-xs text-text/50">
          {averageWords === null
            ? t("settings.postProcessing.api.models.costAssumed", {
                words: ASSUMED_WORDS,
              })
            : t("settings.postProcessing.api.models.costMeasured", {
                words: Math.round(averageWords),
              })}
        </p>
      )}

      {loading && (
        <div className="flex items-center justify-center py-10">
          <Loader2 className="w-6 h-6 animate-spin text-text/40" />
        </div>
      )}

      <div className="space-y-3">
        {!loading &&
          (source === "local"
            ? visibleLocal
                .filter((model) => model.name !== activeLocal?.name)
                .map(localCard)
            : visibleRemote
                .filter((model) => model.id !== activeRemote?.id)
                .map(remoteCard))}
        {isEmpty && (
          <div className="text-center py-8 text-text/50">
            {t("settings.postProcessing.api.models.noMatches")}
          </div>
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

type Fact = { icon: React.ReactNode; text: string } | null;

/**
 * One model, laid out like the transcription model cards: name and state on
 * top, the numbers that decide the choice along the bottom.
 */
const ModelCard: React.FC<{
  title: string;
  subtitle: string;
  facts: Fact[];
  active: boolean;
  activeLabel: string;
  badge?: string;
  onSelect: () => void;
}> = ({ title, subtitle, facts, active, activeLabel, badge, onSelect }) => (
  <button
    type="button"
    onClick={onSelect}
    className={`w-full flex flex-col rounded-xl px-4 py-3 gap-2 text-left transition-all duration-200 ${
      active
        ? "border-2 border-logo-primary/50 bg-logo-primary/10"
        : "border-2 border-mid-gray/20 cursor-pointer hover:border-logo-primary/50 hover:bg-logo-primary/5 hover:shadow-lg hover:scale-[1.01] active:scale-[0.99]"
    }`}
  >
    <div className="flex items-start justify-between gap-3">
      <div className="min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="font-semibold truncate">{title}</span>
          {badge && <Badge variant="secondary">{badge}</Badge>}
        </div>
        {subtitle && subtitle !== title && (
          <p className="text-text/50 text-xs truncate">{subtitle}</p>
        )}
      </div>
      {active && (
        <Badge>
          <Check className="w-3 h-3 me-1" />
          {activeLabel}
        </Badge>
      )}
    </div>

    <div className="flex items-center gap-4 text-xs text-text/60 flex-wrap">
      {facts.filter(Boolean).map((fact, index) => (
        <span key={index} className="flex items-center gap-1.5">
          {fact!.icon}
          {fact!.text}
        </span>
      ))}
    </div>
  </button>
);
