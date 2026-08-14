import React from "react";
import { useTranslation } from "react-i18next";
import { EndpointStatusNotice } from "../PostProcessingSettingsApi/EndpointStatusNotice";
import { ModelPicker } from "../PostProcessingSettingsApi/ModelPicker";
import { usePostProcessProviderState } from "../PostProcessingSettingsApi/usePostProcessProviderState";
import { OPENROUTER_PROVIDER_ID } from "../PostProcessingSettingsApi/types";
import { navigateToSection } from "@/lib/navigate";

/**
 * Refinement models, living next to the transcription models rather than
 * inside the post-processing settings.
 *
 * Both answer the same question — "which model does this job" — and the list
 * needs room to breathe: it had been squeezed into a settings row, where a
 * field or a switch belongs, not a searchable catalogue of several hundred
 * entries.
 *
 * The service status comes along, because it is what explains an empty list:
 * without a running Ollama there is nothing to choose from.
 */
export const RefinementModels: React.FC = () => {
  const { t } = useTranslation();
  const state = usePostProcessProviderState();

  const isLocal = state.selectedProviderId !== OPENROUTER_PROVIDER_ID;

  return (
    <div className="space-y-4">
      {/* Named up top, before anything else.
          Which model is in use is otherwise only visible in the list below —
          and that list shows one source at a time, so standing in "via
          OpenRouter" hid a locally selected model entirely. */}
      <div className="pt-2 pb-1 space-y-0.5">
        <p className="text-sm font-semibold text-text">
          {t("settings.models.selected")}
        </p>
        <p className="text-sm text-text/70">
          {state.model ? (
            <>
              {state.model}{" "}
              <span className="text-text/40">
                {isLocal
                  ? t("settings.models.selectedLocal")
                  : t("settings.models.selectedRemote")}
              </span>
            </>
          ) : (
            t("settings.models.selectedNone")
          )}
        </p>
        {/* The provider is configured one section over. Naming it here — with a
            way to get there — beats making the user remember which of the two
            screens holds which half of the same decision. */}
        <p className="text-xs text-text/50">
          {t("settings.models.viaProvider", {
            provider: state.selectedProvider?.label ?? state.selectedProviderId,
          })}
          <button
            type="button"
            onClick={() => navigateToSection("postprocessing")}
            className="ms-2 underline underline-offset-2 hover:text-text transition-colors"
          >
            {t("settings.models.openSettings")}
          </button>
        </p>
      </div>

      <p className="text-sm text-text/60">
        {t("settings.models.refinementDescription")}
      </p>

      <EndpointStatusNotice
        providerId={state.selectedProviderId}
        baseUrl={state.baseUrl}
      />

      <ModelPicker
        activeProviderId={state.selectedProviderId}
        activeModel={state.model}
        onPick={state.handleModelPick}
      />
    </div>
  );
};
