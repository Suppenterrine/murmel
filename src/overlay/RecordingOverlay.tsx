import { listen } from "@tauri-apps/api/event";
import React, {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import "./RecordingOverlay.css";
import { commands, events } from "@/bindings";
import type {
  StreamPhase,
  StreamPhaseEvent,
  StreamTextEvent,
  StreamWorkKind,
} from "@/bindings";
import i18n, { syncLanguageFromSettings } from "@/i18n";
import { getLanguageDirection } from "@/lib/utils/rtl";

type OverlayState =
  | "recording"
  | "streaming"
  | "transcribing"
  | "processing"
  | "problem";

/** What this dictation is for — see `overlay.rs`. */
type DictationIntent = "plain" | "refined" | "command";

/** A failure worth telling the user about — see `problem.rs`. */
type ProblemReport = {
  /** Translation key suffix under `problem.*`. The backend sends a key rather
   *  than a sentence, so the wording lives with the rest of the translations. */
  key: string;
  detail: string | null;
  /** Whether the history holds something that can be retried. */
  recoverable: boolean;
};

/** How long a problem stays on screen before the overlay gives the desktop back.
 *  Long enough to read two lines, short enough not to sit in the way. */
const PROBLEM_VISIBLE_MS = 8000;

// Number of reactive bars in the waveform (the simple, smoothed style shared by
// every overlay form). Mic levels arrive as 16 FFT buckets; we take the first N.
const WAVE_BARS = 9;

const RecordingOverlay: React.FC = () => {
  const { t } = useTranslation();
  const [isVisible, setIsVisible] = useState(false);
  const [state, setState] = useState<OverlayState>("recording");
  // True when this dictation was started with the refinement hotkey. The two
  // hotkeys look identical while speaking otherwise, and the difference only
  // becomes visible at the very end — or never, if nothing is configured.
  const [postProcess, setPostProcess] = useState(false);
  // Command Mode needs its own label rather than the refinement sparkle: what
  // is being spoken there is an *instruction*, and a user who thinks they are
  // dictating text gets a paragraph replaced by their own sentence.
  const [intent, setIntent] = useState<DictationIntent>("plain");
  // The failure currently on screen. Held rather than passed through `state`
  // because it carries a reason, and the reason is the whole point — "something
  // went wrong" is what the user already knows.
  const [problem, setProblem] = useState<ProblemReport | null>(null);
  const [levels, setLevels] = useState<number[]>(Array(WAVE_BARS).fill(0));
  const [streamText, setStreamText] = useState<StreamTextEvent>({
    committed: "",
    tentative: "",
  });
  const [phase, setPhase] = useState<StreamPhase>("listening");
  const [workKind, setWorkKind] = useState<StreamWorkKind>("transcribing");
  const [elapsed, setElapsed] = useState(0);
  // Bumped on each new streaming session so the Live card remounts fresh (replays
  // the pop-in, and never animates in from the previous panel's open size).
  const [session, setSession] = useState(0);
  // Overlay placement (top vs bottom of the screen). The Live panel grows downward
  // from a top overlay (oldest line under the pill) and upward from a bottom one.
  const [position, setPosition] = useState<"top" | "bottom">("bottom");
  // True once live text overflows the cap. A top overlay fades its top edge only
  // while overflowing, so the resting first line stays crisp flush under the pill.
  const [overflowing, setOverflowing] = useState(false);

  const smoothedLevelsRef = useRef<number[]>(Array(16).fill(0));
  // Live-text scroll-back: the text region "sticks" to the newest line while the
  // user is at the bottom; if they scroll up to read history, auto-follow pauses
  // until they scroll back down.
  const capRef = useRef<HTMLDivElement>(null);
  const pinnedRef = useRef(true);
  const direction = getLanguageDirection(i18n.language);

  useEffect(() => {
    const setupEventListeners = async () => {
      const unlistenShow = await listen("show-overlay", async (event) => {
        await syncLanguageFromSettings();
        // The Live panel flows downward from a top overlay and upward from a
        // bottom one; read the placement so the layout can flip to match.
        try {
          const settings = await commands.getAppSettings();
          if (settings.status === "ok") {
            setPosition(
              settings.data.overlay_position === "top" ? "top" : "bottom",
            );
          }
        } catch {
          // Keep the previous/default placement if settings can't be read.
        }
        const payload = event.payload as {
          state: OverlayState;
          post_process: boolean;
          intent?: DictationIntent;
        };
        const overlayState = payload.state;
        setState(overlayState);
        setPostProcess(payload.post_process);
        setIntent(payload.intent ?? "plain");
        if (overlayState === "recording" || overlayState === "streaming") {
          setStreamText({ committed: "", tentative: "" });
        }
        if (overlayState === "streaming") {
          setPhase("listening");
          setWorkKind("transcribing");
          setElapsed(0);
          setSession((s) => s + 1); // remount the card fresh for this session
        }
        setIsVisible(true);
      });

      const unlistenHide = await listen("hide-overlay", () => {
        // A dictation that failed hides the overlay and reports the problem in
        // the same breath. The hide belongs to the attempt that just ended; the
        // problem is what replaced it, and it takes itself down on a timer.
        setState((current) => {
          if (current !== "problem") setIsVisible(false);
          return current;
        });
      });

      // A problem takes over the overlay: whatever was being shown has failed,
      // so there is nothing left to keep on screen beside it.
      const unlistenProblem = await listen<ProblemReport>(
        "show-problem",
        async (event) => {
          await syncLanguageFromSettings();
          setProblem(event.payload);
          setState("problem");
          setIsVisible(true);
        },
      );

      const unlistenLevel = await listen<number[]>("mic-level", (event) => {
        const newLevels = event.payload as number[];
        // Exponential smoothing across the 16 buckets, then take the first N
        // bars for the shared waveform.
        const smoothed = smoothedLevelsRef.current.map((prev, i) => {
          const target = newLevels[i] || 0;
          return prev * 0.7 + target * 0.3;
        });
        smoothedLevelsRef.current = smoothed;
        setLevels(smoothed.slice(0, WAVE_BARS));
      });

      const unlistenStream = await events.streamTextEvent.listen((event) => {
        setStreamText(event.payload);
      });

      const unlistenPhase = await events.streamPhaseEvent.listen((event) => {
        const payload: StreamPhaseEvent = event.payload;
        setPhase(payload.phase);
        if (payload.kind) setWorkKind(payload.kind);
      });

      return () => {
        unlistenShow();
        unlistenHide();
        unlistenProblem();
        unlistenLevel();
        unlistenStream();
        unlistenPhase();
      };
    };

    setupEventListeners();
  }, []);

  // Elapsed timer while the Live overlay is visible.
  useEffect(() => {
    if (state !== "streaming" || !isVisible) return;
    const id = setInterval(() => setElapsed((e) => e + 1), 1000);
    return () => clearInterval(id);
  }, [state, isVisible]);

  // Takes the message off screen and lets the backend hide the window with it.
  // Both halves are needed: the card fading out leaves a transparent window
  // sitting over the desktop otherwise.
  const dismissProblem = useCallback(() => {
    setIsVisible(false);
    void commands.dismissOverlayProblem();
  }, []);

  // A problem gives the desktop back on its own. Unlike the working states it
  // has nothing following it that would take the overlay down — the dictation it
  // belonged to is over.
  useEffect(() => {
    if (state !== "problem" || !isVisible) return;
    const id = setTimeout(dismissProblem, PROBLEM_VISIBLE_MS);
    return () => clearTimeout(id);
  }, [state, isVisible, problem, dismissProblem]);

  // Stick to the bottom as text streams in — but only while pinned, so a user who
  // has scrolled up to read history isn't yanked back down by the next chunk.
  useLayoutEffect(() => {
    const el = capRef.current;
    if (!el) return;
    // Fade the top edge only once text actually overflows the cap.
    setOverflowing(el.scrollHeight > el.clientHeight + 1);
    if (pinnedRef.current) el.scrollTop = el.scrollHeight;
  }, [streamText]);

  // Each fresh streaming session starts pinned to the bottom, fade cleared.
  useEffect(() => {
    pinnedRef.current = true;
    setOverflowing(false);
  }, [session]);

  if (!isVisible) return null;

  // Re-pin when the user is within ~a line of the bottom; unpin otherwise.
  const handleStreamScroll = () => {
    const el = capRef.current;
    if (!el) return;
    pinnedRef.current = el.scrollHeight - el.scrollTop - el.clientHeight <= 16;
  };

  const fmtTime = (s: number) =>
    `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;

  // ---- Shared building blocks (one visual language for every overlay form) ----
  const waveform = (
    <div className="swave">
      {levels.map((v, i) => (
        <i
          key={i}
          style={{
            height: `${Math.max(3, Math.min(18, 3 + Math.pow(v, 0.7) * 15))}px`,
          }}
        />
      ))}
    </div>
  );

  const cancelBtn = (
    <button
      className="sx"
      aria-label="cancel"
      onClick={() => commands.cancelOperation()}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path
          d="M4 4 L12 12 M12 4 L4 12"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
        />
      </svg>
    </button>
  );

  // dot (left) | waveform (center) | timer + cancel (right) — same structure for
  // pill & panel, so the Live morph is a pure width change.
  const listeningRow = (showTimer: boolean, showCancel: boolean) => (
    <div className="sbase">
      <div className="sbase-l">
        <span className="sdot" />
        {/* Sits beside the dot so the difference between the two dictation
            hotkeys is visible while speaking, not only once the text arrives.
            Same symbol the sidebar uses for refinement. */}
        {/* Command Mode says so in words. The sparkle means "this will be
            polished afterwards"; here the point is that what you are about to
            say is an instruction and not the text itself — that cannot be
            carried by an icon. */}
        {intent === "command" && (
          <span className="sintent">{t("overlay.instruction")}</span>
        )}
        {postProcess && intent !== "command" && (
          <svg
            className="spolish"
            viewBox="0 0 24 24"
            aria-hidden="true"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z" />
          </svg>
        )}
      </div>
      {waveform}
      <div className="sbase-r">
        {showTimer && <span className="stimer">{fmtTime(elapsed)}</span>}
        {showCancel && cancelBtn}
      </div>
    </div>
  );

  // warning sign (left) | reason (center) | dismiss (right).
  //
  // The reason gets the whole middle column and two lines: the user is being
  // told something they did not expect, and "an error occurred" would send them
  // to the log — which is exactly the trip this is meant to save.
  const problemRow = () => (
    <div className="sbase sproblem">
      <div className="sbase-l">
        <svg
          className="sproblem-icon"
          viewBox="0 0 24 24"
          aria-hidden="true"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
          <path d="M12 9v4" />
          <path d="M12 17h.01" />
        </svg>
      </div>
      <div className="sproblem-text">
        <span className="sproblem-title">
          {t(`problem.${problem?.key ?? "unknown"}`, {
            defaultValue: t("problem.unknown"),
          })}
        </span>
        {problem?.recoverable && (
          <span className="sproblem-hint">{t("problem.inHistory")}</span>
        )}
      </div>
      <div className="sbase-r">
        {/* The ring winds down over the time the message has left, so the user
            can see it is going rather than wonder whether it is stuck — and can
            cut it short instead of waiting. */}
        <button
          className="sx sdismiss"
          onClick={dismissProblem}
          title={t("problem.dismiss")}
          aria-label={t("problem.dismiss")}
        >
          {/* Two layers: the ring rides the rim of the button's own circle, the
              cross keeps the size it has everywhere else. Drawing both in one
              SVG made the ring's stroke eat into the glyph. */}
          <svg
            className="sdismiss-ring-svg"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <circle className="sdismiss-track" cx="12" cy="12" r="11" />
            <circle className="sdismiss-ring" cx="12" cy="12" r="11" />
          </svg>
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path
              d="M4 4 L12 12 M12 4 L4 12"
              stroke="currentColor"
              strokeWidth="1.6"
              strokeLinecap="round"
            />
          </svg>
        </button>
      </div>
    </div>
  );

  // spinner (left) | label (center) | cancel (right) — same 3-zone grid as the
  // listening row, so the label is centered.
  const workingRow = (label: string, showCancel: boolean) => (
    <div className="sbase">
      <div className="sbase-l">
        <span className="sspinner" />
      </div>
      <span className="swork-label">{label}</span>
      <div className="sbase-r">{showCancel && cancelBtn}</div>
    </div>
  );

  // ---- Live overlay: a pill that sculpts open into a panel ----
  if (state === "streaming") {
    const hasText =
      streamText.committed.length > 0 || streamText.tentative.length > 0;
    const working = phase === "working";
    // Keep the panel open whenever there's text — even while finalizing — so the
    // transcript stays put under a working spinner instead of collapsing and
    // squishing the text mid-stream. Only fall back to the small working pill
    // when there was no text to preserve.
    const open = hasText;
    const collapsed = working && !hasText;

    return (
      <div dir={direction} className={`ov-stage ${position}`}>
        <div
          key={session}
          className={`scard ${open ? "open" : ""} ${collapsed ? "working" : ""} ${
            isVisible ? "" : "leaving"
          }`}
        >
          <div className="stext">
            <div className="stext-clip">
              <div
                className={`stext-cap ${overflowing ? "overflowing" : ""}`}
                ref={capRef}
                onScroll={handleStreamScroll}
              >
                <p>
                  <span className="committed">
                    {streamText.committed ? streamText.committed + " " : ""}
                  </span>
                  <span className="tentative">{streamText.tentative}</span>
                  {/* Drop the blinking caret once finalizing — it's no longer
                      capturing, and a static spinner conveys the work. */}
                  {!working && <span className="scaret" />}
                </p>
              </div>
            </div>
          </div>
          {working
            ? workingRow(
                workKind === "polishing"
                  ? t("overlay.processing")
                  : t("overlay.transcribing"),
                true,
              )
            : listeningRow(open, true)}
        </div>
      </div>
    );
  }

  // ---- Minimal overlay: exactly one row at a time — waveform (recording), or a
  // spinner + label (transcribing / processing). Never both. The pill animates its
  // width between them; the cancel button is in both rows so it stays put.
  const working = state === "transcribing" || state === "processing";
  const failed = state === "problem";
  const workLabel =
    state === "processing"
      ? intent === "command"
        ? t("overlay.rewriting")
        : t("overlay.processing")
      : t("overlay.transcribing");

  return (
    <div
      dir={direction}
      className={`ov-stage ${position} ov-fade ${isVisible ? "show" : ""}`}
    >
      <div
        className={`scard compact ${working && isVisible ? "cworking" : ""} ${
          failed ? "cproblem" : ""
        } ${
          // Only while listening: the working row has no label beside its
          // spinner, so it needs neither the extra width nor the shifted grid.
          !working && !failed && intent === "command" ? "cintent" : ""
        }`}
      >
        {failed
          ? problemRow()
          : working
            ? workingRow(workLabel, true)
            : listeningRow(false, true)}
      </div>
    </div>
  );
};

export default RecordingOverlay;
