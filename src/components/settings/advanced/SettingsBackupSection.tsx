import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { toast } from "sonner";
import { commands, type RestoreReport, type SettingsBackup } from "@/bindings";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";
import { useSettingsStore } from "../../../stores/settingsStore";

interface SettingsBackupSectionProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const SettingsBackupSection: React.FC<SettingsBackupSectionProps> = ({
  descriptionMode = "inline",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const refreshSettings = useSettingsStore((state) => state.refreshSettings);
  const [backups, setBackups] = useState<SettingsBackup[]>([]);
  const [busy, setBusy] = useState(false);

  const loadBackups = useCallback(async () => {
    const result = await commands.listSettingsBackups();
    if (result.status === "ok") {
      setBackups(result.data);
    } else {
      console.error("Failed to list settings backups:", result.error);
    }
  }, []);

  useEffect(() => {
    void loadBackups();
  }, [loadBackups]);

  // Settings that came back have to reach the UI, and a model the restored
  // settings name but that is not on disk is worth saying out loud — the user
  // would otherwise find out at the next dictation.
  const afterRestore = async (report: RestoreReport) => {
    await refreshSettings();
    await loadBackups();

    if (report.missing_model) {
      toast(t("settings.backup.missingModel", { model: report.missing_model }));
    } else {
      toast.success(t("settings.backup.restored"));
    }
  };

  const exportToFile = async () => {
    setBusy(true);
    try {
      const result = await commands.exportSettings();
      if (result.status !== "ok") {
        toast.error(result.error);
        return;
      }

      const path = await save({
        defaultPath: "murmel-settings.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) return;

      await writeTextFile(path, result.data);
      toast.success(t("settings.backup.exported"));
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(false);
    }
  };

  const importFromFile = async () => {
    setBusy(true);
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path || Array.isArray(path)) return;

      const result = await commands.importSettings(await readTextFile(path));
      if (result.status !== "ok") {
        toast.error(result.error);
        return;
      }

      await afterRestore(result.data);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(false);
    }
  };

  const restoreBackup = async (name: string) => {
    setBusy(true);
    try {
      const result = await commands.restoreSettingsBackup(name);
      if (result.status !== "ok") {
        toast.error(result.error);
        return;
      }

      await afterRestore(result.data);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SettingContainer
      title={t("settings.backup.title")}
      description={t("settings.backup.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
      layout="stacked"
    >
      <div className="space-y-3">
        <div className="flex gap-2">
          <Button variant="secondary" onClick={exportToFile} disabled={busy}>
            {t("settings.backup.export")}
          </Button>
          <Button variant="secondary" onClick={importFromFile} disabled={busy}>
            {t("settings.backup.import")}
          </Button>
        </div>

        <div className="space-y-1">
          <p className="text-xs font-medium text-mid-gray uppercase tracking-wide">
            {t("settings.backup.automatic")}
          </p>
          {backups.length === 0 ? (
            <p className="text-xs text-text/60">{t("settings.backup.none")}</p>
          ) : (
            <ul className="divide-y divide-mid-gray/20">
              {backups.map((backup) => (
                <li
                  key={backup.name}
                  className="flex items-center justify-between py-1.5"
                >
                  <span className="text-sm text-text/80">
                    {backup.taken_at}
                  </span>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => restoreBackup(backup.name)}
                    disabled={busy}
                  >
                    {t("settings.backup.restore")}
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </SettingContainer>
  );
};
