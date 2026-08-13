import React from "react";
import { useTranslation } from "react-i18next";
import { EndpointStatusNotice } from "../PostProcessingSettingsApi/EndpointStatusNotice";
import { ModelPicker } from "../PostProcessingSettingsApi/ModelPicker";
import { usePostProcessProviderState } from "../PostProcessingSettingsApi/usePostProcessProviderState";

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

  return (
    <div className="space-y-4">
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
