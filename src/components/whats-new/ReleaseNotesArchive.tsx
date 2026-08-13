import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ScrollText } from "lucide-react";
import { listReleaseNotes } from "./releaseNotes";
import type { ReleaseNote } from "./releaseNotes";
import { WhatsNewModal } from "./WhatsNewModal";

interface ReleaseNotesArchiveProps {
  /** The running app version, marked in the list. */
  currentVersion: string;
}

/**
 * Lists every release note shipped with this build, newest first.
 *
 * The update window only shows the highest note newer than the last one seen.
 * Updating across two versions therefore skips the older one for good — and a
 * release that shipped without a note (0.11.0 did) would never be readable at
 * all. This is where those can still be found.
 */
export const ReleaseNotesArchive: React.FC<ReleaseNotesArchiveProps> = ({
  currentVersion,
}) => {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<ReleaseNote | null>(null);
  const notes = useMemo(() => listReleaseNotes(), []);

  if (notes.length === 0) return null;

  return (
    <>
      {/* Capped at roughly six entries, then scrolls.
          The list only grows: every release adds a row, and without a ceiling
          the Info page would get taller forever while the interesting entries
          — the recent ones — stay at the top anyway. */}
      <ul className="flex flex-col max-h-64 overflow-y-auto pe-1">
        {notes.map((note) => (
          <li key={note.version}>
            <button
              type="button"
              onClick={() => setSelected(note)}
              className="w-full flex items-center gap-3 px-3 py-2 rounded-md text-left transition-colors cursor-pointer hover:bg-surface-active"
            >
              <ScrollText className="w-4 h-4 shrink-0 text-text/40" />
              <span className="font-medium">
                {t("settings.about.releaseNotes.version", {
                  version: note.version,
                })}
              </span>
              {note.version === currentVersion && (
                <span className="text-xs text-text/50">
                  {t("settings.about.releaseNotes.current")}
                </span>
              )}
            </button>
          </li>
        ))}
      </ul>

      {selected && (
        <WhatsNewModal
          note={selected}
          open={true}
          onDismiss={() => setSelected(null)}
        />
      )}
    </>
  );
};
