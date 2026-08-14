import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronRight, Plus, Search, Trash2 } from "lucide-react";
import { useSettings } from "../../../hooks/useSettings";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";

/**
 * The dictionary: words Murmel should get right.
 *
 * Entries are the *correct* spelling, not a pair. The matcher
 * (`apply_custom_words`) compares each transcribed word against them by edit
 * distance and phonetic similarity, so "Charge B" finds "ChargeBee" without
 * anyone writing down what the misrecognition looked like — and it catches
 * variants nobody thought to list.
 *
 * Multi-word entries are allowed on purpose: the matcher works over n-grams up
 * to three words, and those are exactly the cases it was built for. The old
 * form rejected spaces, which shut that out.
 */
export const DictionarySettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const saved = useMemo(() => getSetting("custom_words") || [], [getSetting]);
  const [draft, setDraft] = useState<string[]>(saved);
  const [newWord, setNewWord] = useState("");
  const [query, setQuery] = useState("");

  // Adopt outside changes (a suggestion accepted from the history, say) as long
  // as nothing is being edited here.
  useEffect(() => {
    setDraft(saved);
  }, [saved]);

  const dirty = useMemo(
    () =>
      draft.length !== saved.length ||
      draft.some((word, index) => word !== saved[index]),
    [draft, saved],
  );

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return draft
      .map((word, index) => ({ word, index }))
      .filter(({ word }) => !needle || word.toLowerCase().includes(needle));
  }, [draft, query]);

  const addWord = () => {
    const word = newWord.trim();
    if (!word || draft.includes(word)) return;
    setDraft((previous) => [...previous, word]);
    setNewWord("");
  };

  const save = () => {
    // Empty rows are the natural result of clearing a field to delete it.
    updateSetting(
      "custom_words",
      draft.map((word) => word.trim()).filter(Boolean),
    );
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-4">
      <div>
        <h1 className="text-xl font-semibold mb-2">
          {t("settings.dictionary.title")}
        </h1>
        <p className="text-sm text-text/60">
          {t("settings.dictionary.description")}
        </p>
      </div>

      {/* Sticky, so the save button stays reachable however long the list is. */}
      <div className="sticky top-0 z-10 -mx-1 px-1 py-2 bg-background/95 backdrop-blur-sm">
        <div className="flex items-center gap-2">
          <Input
            value={newWord}
            onChange={(event) => setNewWord(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") addWord();
            }}
            placeholder={t("settings.dictionary.addPlaceholder")}
            variant="compact"
            className="flex-1"
          />
          <Button
            variant="secondary"
            size="sm"
            onClick={addWord}
            disabled={!newWord.trim()}
          >
            <Plus className="w-3.5 h-3.5 me-1" />
            {t("settings.dictionary.add")}
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={save}
            disabled={!dirty || isUpdating("custom_words")}
          >
            {dirty
              ? t("settings.dictionary.save")
              : t("settings.dictionary.saved")}
          </Button>
        </div>
      </div>

      {draft.length > 8 && (
        <div className="relative">
          <Search className="absolute start-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text/40 pointer-events-none" />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t("settings.dictionary.searchPlaceholder")}
            aria-label={t("settings.dictionary.searchPlaceholder")}
            className="w-full ps-9 pe-3 py-2 text-sm bg-mid-gray/10 border border-mid-gray/40 rounded-lg focus:outline-none focus:ring-1 focus:ring-logo-primary placeholder:text-text/40"
          />
        </div>
      )}

      {draft.length === 0 ? (
        <div className="text-center py-12 text-text/50">
          {t("settings.dictionary.empty")}
        </div>
      ) : (
        <div className="border-2 border-mid-gray/20 rounded-xl divide-y divide-mid-gray/15 overflow-hidden">
          {visible.map(({ word, index }) => (
            <div
              key={index}
              className="flex items-center gap-2 px-3 py-1.5 hover:bg-mid-gray/5 transition-colors"
            >
              <input
                value={word}
                onChange={(event) =>
                  setDraft((previous) =>
                    previous.map((entry, position) =>
                      position === index ? event.target.value : entry,
                    ),
                  )
                }
                className="flex-1 bg-transparent py-1 text-sm focus:outline-none"
                aria-label={t("settings.dictionary.entryLabel", { word })}
              />
              <button
                type="button"
                onClick={() =>
                  setDraft((previous) =>
                    previous.filter((_, position) => position !== index),
                  )
                }
                title={t("settings.dictionary.remove")}
                aria-label={t("settings.dictionary.remove")}
                className="p-1.5 rounded-md text-text/40 hover:text-error transition-colors"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            </div>
          ))}

          {visible.length === 0 && (
            <div className="px-4 py-6 text-center text-sm text-text/50">
              {t("settings.dictionary.noMatches")}
            </div>
          )}
        </div>
      )}

      <p className="text-xs text-text/50">{t("settings.dictionary.hint")}</p>

      {/*
       * Folded away rather than printed above the list: it answers questions
       * ("do I have to enter the misheard version too?") that only come up once,
       * and a permanent wall of explanation over a five-line list gets the
       * weighting backwards.
       */}
      <details className="group border-t border-mid-gray/20 pt-3">
        <summary className="text-xs text-text/50 hover:text-text/80 cursor-pointer list-none flex items-center gap-1.5 transition-colors">
          <ChevronRight className="w-3.5 h-3.5 transition-transform group-open:rotate-90" />
          {t("settings.dictionary.howTitle")}
        </summary>
        <div className="mt-3 space-y-2 text-xs text-text/60 leading-relaxed ps-5">
          <p>{t("settings.dictionary.howBefore")}</p>
          <p>{t("settings.dictionary.howAfter")}</p>
          <p>{t("settings.dictionary.howPhrases")}</p>
          <p className="text-text/45">{t("settings.dictionary.howLimits")}</p>
        </div>
      </details>
    </div>
  );
};
