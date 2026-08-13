import { Moon, Sun } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useSettings } from "@/hooks/useSettings";
import { applyTheme } from "@/lib/utils/theme";
import type { Theme } from "@/bindings";

/**
 * Schneller Hell/Dunkel-Umschalter für die Fußzeile.
 *
 * Ergänzt das Dropdown in den Einstellungen, das zusätzlich „System" kennt.
 * Steht die Einstellung auf `system`, wird zuerst ermittelt, welche Palette
 * gerade tatsächlich greift, und dann auf die andere gewechselt — sonst
 * bliebe der erste Klick scheinbar wirkungslos.
 */
export const ThemeToggle = () => {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettings();
  const setting: Theme = settings?.theme ?? "light";

  const effectiveIsDark =
    setting === "dark" ||
    (setting === "system" &&
      typeof window !== "undefined" &&
      window.matchMedia?.("(prefers-color-scheme: dark)").matches);

  const toggle = () => {
    const next: Theme = effectiveIsDark ? "light" : "dark";
    applyTheme(next);
    updateSetting("theme", next);
  };

  return (
    <button
      type="button"
      onClick={toggle}
      title={t("theme.title")}
      aria-label={t("theme.title")}
      className="flex items-center justify-center h-6 w-6 rounded-md text-text/60 hover:text-text hover:bg-mid-gray/20 transition-colors cursor-pointer"
    >
      {effectiveIsDark ? <Sun size={14} /> : <Moon size={14} />}
    </button>
  );
};

export default ThemeToggle;
