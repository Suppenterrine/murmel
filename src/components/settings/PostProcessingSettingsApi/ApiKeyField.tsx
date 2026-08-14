import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, KeyRound, Loader2 } from "lucide-react";
import { commands } from "@/bindings";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";

interface ApiKeyFieldProps {
  providerId: string;
  disabled?: boolean;
  placeholder?: string;
}

/**
 * One line, three states: stored → editing → stored again.
 *
 * The key itself never comes back from the backend — it lives in the system
 * credential store and the frontend only ever learns *that* one exists. So
 * there is nothing to prefill a field with, and showing an empty password box
 * next to a working configuration would suggest the key was gone.
 *
 * Hence the cycle: a stored key shows as a note with a button, and the field
 * appears only after the user asks for it.
 */
export const ApiKeyField: React.FC<ApiKeyFieldProps> = ({
  providerId,
  disabled,
  placeholder,
}) => {
  const { t } = useTranslation();
  const [hasKey, setHasKey] = useState<boolean | null>(null);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [justSaved, setJustSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const result = await commands.hasPostProcessApiKey(providerId);
      setHasKey(result.status === "ok" ? result.data : false);
    } catch {
      setHasKey(false);
    }
  }, [providerId]);

  useEffect(() => {
    setEditing(false);
    setDraft("");
    setError(null);
    void refresh();
  }, [refresh]);

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      const result = await commands.changePostProcessApiKeySetting(
        providerId,
        draft,
      );
      if (result.status === "ok") {
        setDraft("");
        setEditing(false);
        await refresh();
        // Brief confirmation, then back to the resting state — a permanent
        // "saved" badge would say nothing a moment later.
        setJustSaved(true);
        setTimeout(() => setJustSaved(false), 2500);
      } else {
        setError(String(result.error));
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    setSaving(true);
    setError(null);
    try {
      const result = await commands.deletePostProcessApiKey(providerId);
      if (result.status !== "ok") setError(String(result.error));
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
      setEditing(false);
      setDraft("");
    }
  };

  // Still asking the credential store — an empty field flashing by would read
  // as "no key stored".
  if (hasKey === null) {
    return <Loader2 className="w-4 h-4 animate-spin text-text/30" />;
  }

  if (hasKey && !editing) {
    return (
      <div className="flex flex-col items-end gap-1">
        <div className="flex items-center gap-3">
          <span className="flex items-center gap-1.5 text-sm text-text/60">
            {justSaved ? (
              <Check className="w-3.5 h-3.5 text-logo-primary" />
            ) : (
              <KeyRound className="w-3.5 h-3.5" />
            )}
            {justSaved
              ? t("settings.postProcessing.api.apiKey.saved")
              : t("settings.postProcessing.api.apiKey.stored")}
          </span>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setEditing(true)}
            disabled={disabled || saving}
          >
            {t("settings.postProcessing.api.apiKey.replace")}
          </Button>
        </div>
        <button
          type="button"
          onClick={remove}
          disabled={disabled || saving}
          className="text-xs text-text/40 hover:text-error transition-colors disabled:opacity-50"
        >
          {t("settings.postProcessing.api.apiKey.remove")}
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col items-end gap-1">
      <div className="flex items-center gap-2">
        <Input
          type="password"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && draft.trim()) void save();
            if (event.key === "Escape" && hasKey) {
              setEditing(false);
              setDraft("");
            }
          }}
          placeholder={placeholder}
          variant="compact"
          disabled={disabled || saving}
          autoFocus={editing}
          className="min-w-[280px]"
        />
        <Button
          variant="primary"
          size="sm"
          onClick={save}
          disabled={disabled || saving || !draft.trim()}
        >
          {saving
            ? t("settings.postProcessing.api.apiKey.saving")
            : t("settings.postProcessing.api.apiKey.save")}
        </Button>
      </div>

      {error && (
        <span className="text-xs text-error max-w-[380px]">{error}</span>
      )}

      {hasKey && (
        <button
          type="button"
          onClick={() => {
            setEditing(false);
            setDraft("");
            setError(null);
          }}
          className="text-xs text-text/40 hover:text-text transition-colors"
        >
          {t("common.cancel")}
        </button>
      )}
    </div>
  );
};

ApiKeyField.displayName = "ApiKeyField";
