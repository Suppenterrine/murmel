import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import MurmelHand from "./icons/MurmelHand";
import MurmelTextLogo from "./icons/MurmelTextLogo";
import Copyright from "./Copyright";

const VISIBLE_MS = 1500;
const FADE_MS = 450;

/**
 * Startbild: Bildmarke, Wortmarke, Rechtevermerk.
 *
 * Zeigt sich nur, wenn das Fenster beim Start tatsächlich sichtbar ist. Murmel
 * startet je nach Einstellung (oder mit `--start-hidden`) in den Tray — dort
 * liefe das Startbild unsichtbar ab und wäre beim späteren Öffnen des Fensters
 * schon vorbei. In dem Fall wird es übersprungen.
 *
 * Ein Klick blendet es sofort aus; niemand soll auf eine Animation warten.
 */
export const Splash = () => {
  const [phase, setPhase] = useState<"checking" | "shown" | "fading" | "done">(
    "checking",
  );

  useEffect(() => {
    let cancelled = false;
    const timers: ReturnType<typeof setTimeout>[] = [];

    (async () => {
      let visible = true;
      try {
        visible = await getCurrentWindow().isVisible();
      } catch {
        // Kein Tauri-Fenster (z. B. im Browser-Dev-Server) — dann einfach zeigen.
      }
      if (cancelled) return;
      if (!visible) {
        setPhase("done");
        return;
      }
      setPhase("shown");
      timers.push(
        setTimeout(() => !cancelled && setPhase("fading"), VISIBLE_MS),
      );
      timers.push(
        setTimeout(() => !cancelled && setPhase("done"), VISIBLE_MS + FADE_MS),
      );
    })();

    return () => {
      cancelled = true;
      timers.forEach(clearTimeout);
    };
  }, []);

  if (phase === "done" || phase === "checking") return null;

  return (
    <div
      onClick={() => setPhase("done")}
      className={`fixed inset-0 z-[9999] flex flex-col items-center justify-center gap-6 bg-background transition-opacity duration-[450ms] ${
        phase === "fading" ? "opacity-0" : "opacity-100"
      }`}
    >
      <MurmelHand width={132} height={132} />
      <MurmelTextLogo width={190} />
      <Copyright />
    </div>
  );
};

export default Splash;
