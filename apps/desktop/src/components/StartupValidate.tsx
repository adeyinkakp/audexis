import { useEffect, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ErrorPage } from "./ErrorPage";
import { logError } from "../utils/logger";

type StartupError = { message: string; details: string };

export function StartupValidate({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<"loading" | "ready" | StartupError>(
    "loading",
  );
  const [relaunching, setRelaunching] = useState(false);
  const [relaunchError, setRelaunchError] = useState<string>();

  useEffect(() => {
    let disposed = false;
    void invoke<StartupError | null>("get_startup_error").then(
      (error) => {
        if (!disposed) setStatus(error ?? "ready");
      },
      (error) => {
        if (!disposed)
          setStatus({
            message: "Audexis could not check whether startup completed.",
            details: String(error),
          });
      },
    );
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    if (typeof status === "object") {
      void getCurrentWindow()
        .show()
        .catch((error) => logError("show startup error", error));
    }
  }, [status]);

  async function relaunch() {
    setRelaunching(true);
    setRelaunchError(undefined);
    try {
      await invoke("relaunch_app");
    } catch (error) {
      logError("Relaunch failed", error);
      setRelaunchError(
        "Audexis could not relaunch. Close the app and open it again.",
      );
      setRelaunching(false);
    }
  }

  if (status === "ready") return children;
  if (status === "loading") return null;
  return (
    <>
      <div data-tauri-drag-region className="fixed inset-x-0 top-0 h-14" />
      <ErrorPage
        kind="startup"
        message={status.message}
        error={status.details}
        onRetry={() => void relaunch()}
        retryPending={relaunching}
        retryError={relaunchError}
      />
    </>
  );
}
