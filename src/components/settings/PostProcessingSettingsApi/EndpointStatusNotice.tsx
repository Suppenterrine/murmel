import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type EndpointStatus } from "@/bindings";
import { Alert } from "../../ui/Alert";
import { Button } from "../../ui/Button";

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
  const [isStarting, setIsStarting] = useState(false);
  const [isStopping, setIsStopping] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);

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

  const startService = useCallback(async () => {
    setIsStarting(true);
    setStartError(null);
    try {
      const result = await commands.startLocalLlmService(providerId);
      if (result.status === "ok") {
        await check();
      } else {
        setStartError(String(result.error));
      }
    } catch (error) {
      setStartError(String(error));
    } finally {
      setIsStarting(false);
    }
  }, [providerId, check]);

  const stopService = useCallback(async () => {
    setIsStopping(true);
    setStartError(null);
    try {
      const result = await commands.stopLocalLlmService();
      if (result.status !== "ok") {
        setStartError(String(result.error));
      }
      await check();
    } catch (error) {
      setStartError(String(error));
    } finally {
      setIsStopping(false);
    }
  }, [check]);

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
        {startError ? (
          <span className="block mt-1 font-mono text-xs opacity-70">
            {startError}
          </span>
        ) : null}
        <div className="mt-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={startService}
            disabled={isStarting}
          >
            {isStarting
              ? t("settings.postProcessing.api.endpoint.starting")
              : t("settings.postProcessing.api.endpoint.start")}
          </Button>
        </div>
      </Alert>
    );
  }

  if (status.reachable === true) {
    return (
      <Alert variant="success" contained>
        <div className="flex items-center justify-between gap-3">
          <span>
            {isChecking
              ? t("settings.postProcessing.api.endpoint.checking")
              : t("settings.postProcessing.api.endpoint.reachable", {
                  count: status.models.length,
                })}
            {/* Who started it matters: a background process Murmel cannot
                account for is exactly the invisible kind we want to avoid. */}
            <span className="block text-xs opacity-70">
              {status.owned_pid
                ? t("settings.postProcessing.api.endpoint.ownedByMurmel", {
                    pid: status.owned_pid,
                  })
                : t("settings.postProcessing.api.endpoint.startedElsewhere")}
            </span>
          </span>
          {status.owned_pid ? (
            <Button
              variant="secondary"
              size="sm"
              onClick={stopService}
              disabled={isStopping}
            >
              {isStopping
                ? t("settings.postProcessing.api.endpoint.stopping")
                : t("settings.postProcessing.api.endpoint.stop")}
            </Button>
          ) : null}
        </div>
      </Alert>
    );
  }

  return null;
};
