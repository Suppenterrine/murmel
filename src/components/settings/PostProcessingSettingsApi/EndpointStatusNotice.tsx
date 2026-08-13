import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type EndpointStatus } from "@/bindings";
import { Alert } from "../../ui/Alert";

type EndpointStatusNoticeProps = {
  providerId: string;
  /** Re-checks when the endpoint is edited, not just when it is switched. */
  baseUrl: string;
};

/**
 * Says where the dictated text goes, and whether a local service is actually
 * running.
 *
 * Both halves earn their place. A cloud provider means the text leaves the
 * machine, which Murmel has to state plainly rather than bury in a dropdown
 * (Northstar §5.2). And a stopped Ollama is otherwise invisible: dictation just
 * quietly arrives unrefined, which reads as "the feature does nothing" instead
 * of "the service is not running".
 */
export const EndpointStatusNotice: React.FC<EndpointStatusNoticeProps> = ({
  providerId,
  baseUrl,
}) => {
  const { t } = useTranslation();
  const [status, setStatus] = useState<EndpointStatus | null>(null);
  const [isChecking, setIsChecking] = useState(false);

  const check = useCallback(async () => {
    if (!providerId) return;

    setIsChecking(true);
    try {
      const result = await commands.checkPostProcessEndpoint(providerId);
      setStatus(result.status === "ok" ? result.data : null);
    } catch (error) {
      console.error("Failed to check post-process endpoint:", error);
      setStatus(null);
    } finally {
      setIsChecking(false);
    }
  }, [providerId]);

  useEffect(() => {
    void check();
  }, [check, baseUrl]);

  if (!status) return null;

  if (!status.is_local) {
    return (
      <Alert variant="warning" contained>
        {t("settings.postProcessing.api.endpoint.remote")}
      </Alert>
    );
  }

  if (status.reachable === false) {
    return (
      <Alert variant="error" contained>
        {t("settings.postProcessing.api.endpoint.unreachable")}
        {status.error ? (
          <span className="block mt-1 font-mono text-xs opacity-70">
            {status.error}
          </span>
        ) : null}
      </Alert>
    );
  }

  if (status.reachable === true) {
    return (
      <Alert variant="success" contained>
        {isChecking
          ? t("settings.postProcessing.api.endpoint.checking")
          : t("settings.postProcessing.api.endpoint.reachable", {
              count: status.models.length,
            })}
      </Alert>
    );
  }

  return null;
};
