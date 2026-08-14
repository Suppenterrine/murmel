import React, { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { readFile } from "@tauri-apps/plugin-fs";
import {
  Check,
  Copy,
  FolderOpen,
  Pencil,
  Plus,
  RotateCcw,
  Search,
  Star,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  commands,
  events,
  type HistoryEntry,
  type HistoryUpdatePayload,
} from "@/bindings";
import { useOsType } from "@/hooks/useOsType";
import { formatDateTime } from "@/utils/dateFormat";
import { AudioPlayer, AudioPlayerGroup } from "../../ui/AudioPlayer";
import { Button } from "../../ui/Button";
import { useSettings } from "../../../hooks/useSettings";

const IconButton: React.FC<{
  onClick: () => void;
  title: string;
  disabled?: boolean;
  active?: boolean;
  children: React.ReactNode;
}> = ({ onClick, title, disabled, active, children }) => (
  <button
    onClick={onClick}
    disabled={disabled}
    className={`p-1.5 rounded-md flex items-center justify-center transition-colors cursor-pointer disabled:cursor-not-allowed disabled:text-text/20 ${
      active
        ? "text-logo-primary hover:text-logo-primary/80"
        : "text-text/50 hover:text-logo-primary"
    }`}
    title={title}
  >
    {children}
  </button>
);

const PAGE_SIZE = 30;

interface OpenRecordingsButtonProps {
  onClick: () => void;
  label: string;
}

const OpenRecordingsButton: React.FC<OpenRecordingsButtonProps> = ({
  onClick,
  label,
}) => (
  <Button
    onClick={onClick}
    variant="secondary"
    size="sm"
    className="flex items-center gap-2"
    title={label}
  >
    <FolderOpen className="w-4 h-4" />
    <span>{label}</span>
  </Button>
);

export const HistorySettings: React.FC = () => {
  const { t } = useTranslation();
  const osType = useOsType();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasMore, setHasMore] = useState(true);
  const [search, setSearch] = useState("");
  const [onlySaved, setOnlySaved] = useState(false);
  // Any active filter switches from cursor paging to a ranked search, so the
  // two never have to agree on what a cursor means.
  const isFiltered = search.trim().length > 0 || onlySaved;
  const sentinelRef = useRef<HTMLDivElement>(null);
  const entriesRef = useRef<HistoryEntry[]>([]);
  const loadingRef = useRef(false);

  // Keep ref in sync for use in IntersectionObserver callback
  useEffect(() => {
    entriesRef.current = entries;
  }, [entries]);

  const loadPage = useCallback(async (cursor?: number) => {
    const isFirstPage = cursor === undefined;
    if (!isFirstPage && loadingRef.current) return;
    loadingRef.current = true;

    if (isFirstPage) setLoading(true);

    try {
      const result = await commands.getHistoryEntries(
        cursor ?? null,
        PAGE_SIZE,
      );
      if (result.status === "ok") {
        const { entries: newEntries, has_more } = result.data;
        setEntries((prev) =>
          isFirstPage ? newEntries : [...prev, ...newEntries],
        );
        setHasMore(has_more);
      }
    } catch (error) {
      console.error("Failed to load history entries:", error);
    } finally {
      setLoading(false);
      loadingRef.current = false;
    }
  }, []);

  // Initial load — only while unfiltered; the search effect below owns the
  // list as soon as a filter is set.
  useEffect(() => {
    if (isFiltered) return;
    loadPage();
  }, [loadPage, isFiltered]);

  // Filtered view. Debounced because it runs on every keystroke, and a search
  // that fires per character makes the list flicker on the way to a word.
  useEffect(() => {
    if (!isFiltered) return;

    let cancelled = false;
    const timer = setTimeout(async () => {
      setLoading(true);
      try {
        const result = await commands.searchHistoryEntries(
          search.trim() || null,
          onlySaved,
          100,
        );
        if (cancelled) return;
        if (result.status === "ok") {
          setEntries(result.data);
          // A search returns its whole result at once — nothing left to page.
          setHasMore(false);
        }
      } catch (error) {
        console.error("Failed to search history entries:", error);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }, 200);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [search, onlySaved, isFiltered]);

  // Infinite scroll via IntersectionObserver
  useEffect(() => {
    if (loading || isFiltered) return;

    const sentinel = sentinelRef.current;
    if (!sentinel || !hasMore) return;

    const observer = new IntersectionObserver(
      (observerEntries) => {
        const first = observerEntries[0];
        if (first.isIntersecting) {
          const lastEntry = entriesRef.current[entriesRef.current.length - 1];
          if (lastEntry) {
            loadPage(lastEntry.id);
          }
        }
      },
      { threshold: 0 },
    );

    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [loading, hasMore, loadPage]);

  // Listen for new entries added from the transcription pipeline
  useEffect(() => {
    const unlisten = events.historyUpdatePayload.listen((event) => {
      const payload: HistoryUpdatePayload = event.payload;
      if (payload.action === "added") {
        setEntries((prev) => [payload.entry, ...prev]);
      } else if (payload.action === "updated") {
        setEntries((prev) =>
          prev.map((e) => (e.id === payload.entry.id ? payload.entry : e)),
        );
      }
      // "deleted" and "toggled" are handled by optimistic updates only,
      // so we intentionally ignore them here to avoid double-mutation.
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const toggleSaved = async (id: number) => {
    // Optimistic update
    setEntries((prev) =>
      prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
    );
    try {
      const result = await commands.toggleHistoryEntrySaved(id);
      if (result.status !== "ok") {
        // Revert on failure
        setEntries((prev) =>
          prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
        );
      }
    } catch (error) {
      console.error("Failed to toggle saved status:", error);
      // Revert on failure
      setEntries((prev) =>
        prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
      );
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch (error) {
      console.error("Failed to copy to clipboard:", error);
    }
  };

  const getAudioUrl = useCallback(
    async (fileName: string) => {
      try {
        const result = await commands.getAudioFilePath(fileName);
        if (result.status === "ok") {
          if (osType === "linux") {
            const fileData = await readFile(result.data);
            const blob = new Blob([fileData], { type: "audio/wav" });
            return URL.createObjectURL(blob);
          }
          return convertFileSrc(result.data, "asset");
        }
        return null;
      } catch (error) {
        console.error("Failed to get audio file path:", error);
        return null;
      }
    },
    [osType],
  );

  const deleteAudioEntry = async (id: number) => {
    // Optimistically remove
    setEntries((prev) => prev.filter((e) => e.id !== id));
    try {
      const result = await commands.deleteHistoryEntry(id);
      if (result.status !== "ok") {
        // Reload on failure
        loadPage();
      }
    } catch (error) {
      console.error("Failed to delete entry:", error);
      loadPage();
    }
  };

  const retryHistoryEntry = async (id: number) => {
    const result = await commands.retryHistoryEntryTranscription(id);
    if (result.status !== "ok") {
      throw new Error(String(result.error));
    }
  };

  const openRecordingsFolder = async () => {
    try {
      const result = await commands.openRecordingsFolder();
      if (result.status !== "ok") {
        throw new Error(String(result.error));
      }
    } catch (error) {
      console.error("Failed to open recordings folder:", error);
    }
  };

  let content: React.ReactNode;

  if (loading) {
    content = (
      <div className="px-4 py-3 text-center text-text/60">
        {t("settings.history.loading")}
      </div>
    );
  } else if (entries.length === 0) {
    content = (
      <div className="px-4 py-3 text-center text-text/60">
        {isFiltered
          ? t("settings.history.noMatches")
          : t("settings.history.empty")}
      </div>
    );
  } else {
    content = (
      <>
        <AudioPlayerGroup>
          <div className="divide-y divide-mid-gray/20">
            {entries.map((entry) => (
              <HistoryEntryComponent
                key={entry.id}
                entry={entry}
                onToggleSaved={() => toggleSaved(entry.id)}
                onCopyText={() => copyToClipboard(entry.transcription_text)}
                getAudioUrl={getAudioUrl}
                deleteAudio={deleteAudioEntry}
                retryTranscription={retryHistoryEntry}
              />
            ))}
          </div>
        </AudioPlayerGroup>
        {/* Sentinel for infinite scroll */}
        <div ref={sentinelRef} className="h-1" />
      </>
    );
  }

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <div className="space-y-2">
        <div className="px-4 flex items-center justify-between">
          <div>
            <h2 className="text-xs font-medium text-mid-gray uppercase tracking-wide">
              {t("settings.history.title")}
            </h2>
          </div>
          <OpenRecordingsButton
            onClick={openRecordingsFolder}
            label={t("settings.history.openFolder")}
          />
        </div>

        <div className="px-4 flex items-center gap-2">
          <div className="relative flex-1">
            <Search className="absolute start-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text/40 pointer-events-none" />
            <input
              type="search"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder={t("settings.history.searchPlaceholder")}
              aria-label={t("settings.history.searchPlaceholder")}
              className="w-full ps-9 pe-3 py-2 text-sm rounded-md bg-background border border-mid-gray/20 text-text placeholder:text-text/40 focus:outline-none focus:border-logo-primary/50"
            />
          </div>
          <IconButton
            onClick={() => setOnlySaved((previous) => !previous)}
            title={t("settings.history.onlyFavorites")}
            active={onlySaved}
          >
            <Star className={`w-4 h-4 ${onlySaved ? "fill-current" : ""}`} />
          </IconButton>
        </div>

        <div className="bg-background border border-mid-gray/20 rounded-lg overflow-visible">
          {content}
        </div>
      </div>
    </div>
  );
};

interface HistoryEntryProps {
  entry: HistoryEntry;
  onToggleSaved: () => void;
  onCopyText: () => void;
  getAudioUrl: (fileName: string) => Promise<string | null>;
  deleteAudio: (id: number) => Promise<void>;
  retryTranscription: (id: number) => Promise<void>;
}

const HistoryEntryComponent: React.FC<HistoryEntryProps> = ({
  entry,
  onToggleSaved,
  onCopyText,
  getAudioUrl,
  deleteAudio,
  retryTranscription,
}) => {
  const { t, i18n } = useTranslation();
  const [showCopied, setShowCopied] = useState(false);
  const [retrying, setRetrying] = useState(false);

  const hasTranscription = entry.transcription_text.trim().length > 0;

  const handleLoadAudio = useCallback(
    () => getAudioUrl(entry.file_name),
    [getAudioUrl, entry.file_name],
  );

  const handleCopyText = () => {
    if (!hasTranscription) {
      return;
    }

    onCopyText();
    setShowCopied(true);
    setTimeout(() => setShowCopied(false), 2000);
  };

  const handleDeleteEntry = async () => {
    try {
      await deleteAudio(entry.id);
    } catch (error) {
      console.error("Failed to delete entry:", error);
      toast.error(t("settings.history.deleteError"));
    }
  };

  const handleRetranscribe = async () => {
    try {
      setRetrying(true);
      await retryTranscription(entry.id);
    } catch (error) {
      console.error("Failed to re-transcribe:", error);
      toast.error(t("settings.history.retranscribeError"));
    } finally {
      setRetrying(false);
    }
  };

  const formattedDate = formatDateTime(String(entry.timestamp), i18n.language);

  return (
    <div className="px-4 py-2 pb-5 flex flex-col gap-3">
      <div className="flex justify-between items-center">
        <p className="text-sm font-medium">{formattedDate}</p>
        <div className="flex items-center">
          <IconButton
            onClick={handleCopyText}
            disabled={!hasTranscription || retrying}
            title={t("settings.history.copyToClipboard")}
          >
            {showCopied ? (
              <Check width={16} height={16} />
            ) : (
              <Copy width={16} height={16} />
            )}
          </IconButton>
          <IconButton
            onClick={onToggleSaved}
            disabled={retrying}
            active={entry.saved}
            title={
              entry.saved
                ? t("settings.history.unsave")
                : t("settings.history.save")
            }
          >
            <Star
              width={16}
              height={16}
              fill={entry.saved ? "currentColor" : "none"}
            />
          </IconButton>
          <IconButton
            onClick={handleRetranscribe}
            disabled={retrying}
            title={t("settings.history.retranscribe")}
          >
            <RotateCcw
              width={16}
              height={16}
              style={
                retrying
                  ? { animation: "spin 1s linear infinite reverse" }
                  : undefined
              }
            />
          </IconButton>
          <IconButton
            onClick={handleDeleteEntry}
            disabled={retrying}
            title={t("settings.history.delete")}
          >
            <Trash2 width={16} height={16} />
          </IconButton>
        </div>
      </div>

      <p
        className={`italic text-sm pb-2 ${
          retrying
            ? ""
            : hasTranscription
              ? "text-text/90 select-text cursor-text whitespace-pre-wrap break-words"
              : "text-text/40"
        }`}
        style={
          retrying
            ? { animation: "transcribe-pulse 3s ease-in-out infinite" }
            : undefined
        }
      >
        {retrying && (
          <style>{`
            @keyframes transcribe-pulse {
              0%, 100% { color: color-mix(in srgb, var(--color-text) 40%, transparent); }
              50% { color: color-mix(in srgb, var(--color-text) 90%, transparent); }
            }
          `}</style>
        )}
        {retrying
          ? t("settings.history.transcribing")
          : hasTranscription
            ? entry.transcription_text
            : t("settings.history.transcriptionFailed")}
      </p>

      {hasTranscription && !retrying && (
        <TranscriptCorrection id={entry.id} text={entry.transcription_text} />
      )}

      <EntryMetrics entry={entry} />

      <AudioPlayer onLoadRequest={handleLoadAudio} className="w-full" />
    </div>
  );
};

/**
 * Correcting a transcript — and teaching the dictionary from it.
 *
 * This is where words Murmel gets wrong become words it gets right. The
 * alternative would be reading the corrections out of whatever program the text
 * was pasted into, which means a dictation tool watching foreign text fields;
 * doing it here costs one extra step and no surveillance.
 *
 * Suggestions are offered, never adopted automatically: a correction can carry
 * a typo of its own, and a dictionary entry learned from one would bend every
 * later dictation towards it.
 */
const TranscriptCorrection: React.FC<{ id: number; text: string }> = ({
  id,
  text,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(text);
  const [saving, setSaving] = useState(false);
  const [suggestions, setSuggestions] = useState<string[]>([]);

  useEffect(() => {
    setDraft(text);
  }, [text]);

  const save = async () => {
    setSaving(true);
    try {
      const result = await commands.correctHistoryEntry(id, draft);
      if (result.status === "ok") {
        setSuggestions(result.data);
        setEditing(false);
      } else {
        toast.error(String(result.error));
      }
    } catch (error) {
      toast.error(String(error));
    } finally {
      setSaving(false);
    }
  };

  const adopt = (word: string) => {
    const known = getSetting("custom_words") || [];
    if (!known.includes(word)) {
      updateSetting("custom_words", [...known, word]);
    }
    setSuggestions((previous) => previous.filter((entry) => entry !== word));
    toast.success(t("settings.history.correction.added", { word }));
  };

  if (suggestions.length > 0) {
    return (
      <div className="flex flex-wrap items-center gap-2 text-xs">
        <span className="text-text/60">
          {t("settings.history.correction.learn")}
        </span>
        {suggestions.map((word) => (
          <button
            key={word}
            type="button"
            onClick={() => adopt(word)}
            className="px-2 py-1 rounded-md bg-logo-primary/15 text-logo-primary hover:bg-logo-primary/25 transition-colors"
          >
            <Plus className="w-3 h-3 inline me-1" />
            {word}
          </button>
        ))}
        <button
          type="button"
          onClick={() => setSuggestions([])}
          className="text-text/40 hover:text-text transition-colors"
        >
          {t("settings.history.correction.dismiss")}
        </button>
      </div>
    );
  }

  if (!editing) {
    return (
      <button
        type="button"
        onClick={() => setEditing(true)}
        className="self-start text-xs text-text/40 hover:text-logo-primary transition-colors"
      >
        <Pencil className="w-3 h-3 inline me-1" />
        {t("settings.history.correction.edit")}
      </button>
    );
  }

  return (
    <div className="space-y-2">
      <textarea
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        rows={Math.min(Math.ceil(draft.length / 70) + 1, 8)}
        className="w-full px-3 py-2 text-sm rounded-md bg-background border border-mid-gray/40 text-text focus:outline-none focus:border-logo-primary/50 resize-none"
        autoFocus
      />
      <div className="flex items-center gap-2">
        <Button
          variant="primary"
          size="sm"
          onClick={save}
          disabled={saving || draft.trim() === text.trim()}
        >
          {t("settings.history.correction.save")}
        </Button>
        <button
          type="button"
          onClick={() => {
            setEditing(false);
            setDraft(text);
          }}
          className="text-xs text-text/40 hover:text-text transition-colors"
        >
          {t("common.cancel")}
        </button>
      </div>
    </div>
  );
};

/**
 * The numbers behind one dictation: how fast it was spoken, how long the model
 * took, and what happened during refinement.
 *
 * Everything is optional — entries from before these were recorded simply show
 * less, rather than a row of dashes.
 */
const EntryMetrics: React.FC<{ entry: HistoryEntry }> = ({ entry }) => {
  const { t } = useTranslation();

  const parts: string[] = [];

  if (entry.duration_ms && entry.duration_ms > 0) {
    parts.push(
      t("settings.history.metrics.duration", {
        seconds: (entry.duration_ms / 1000).toFixed(1),
      }),
    );

    if (entry.word_count) {
      // Words per minute — the number that says whether dictating is actually
      // faster than typing.
      const wpm = Math.round(entry.word_count / (entry.duration_ms / 60000));
      parts.push(t("settings.history.metrics.wpm", { wpm }));
    }
  }

  if (entry.processing_ms && entry.processing_ms > 0) {
    parts.push(
      t("settings.history.metrics.processing", {
        seconds: (entry.processing_ms / 1000).toFixed(1),
      }),
    );
  }

  if (entry.model_used) parts.push(entry.model_used);

  const run = entry.last_post_process;
  const refinementFailed = run != null && !run.succeeded;

  if (parts.length === 0 && !refinementFailed) return null;

  return (
    <p className="text-xs text-text/40 flex flex-wrap items-center gap-x-2 gap-y-1">
      {parts.map((part, index) => (
        <span key={part}>
          {index > 0 && <span className="me-2 text-text/25">·</span>}
          {part}
        </span>
      ))}
      {refinementFailed && (
        <span className="text-warning" title={run?.error ?? undefined}>
          {t("settings.history.metrics.refinementFailed")}
        </span>
      )}
    </p>
  );
};
