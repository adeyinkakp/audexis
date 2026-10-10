import { useEffect, useSyncExternalStore } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import { Modal } from "./Modal";
import { createUpdateSession } from "../utils/updateSession";
import { logError } from "../utils/logger";

const session = createUpdateSession({
  check: () => check({ timeout: 15000 }),
  restart: () => invoke("relaunch_app"),
  log: logError,
});

export function AppUpdater() {
  const state = useSyncExternalStore(session.subscribe, session.getSnapshot);
  useEffect(() => {
    if (isTauri() && !import.meta.env.DEV) void session.check();
  }, []);
  const busy = state.phase === "downloading" || state.phase === "installing";
  const open = state.phase !== "idle" && state.phase !== "dismissed";

  return (
    <Modal
      open={open}
      onClose={session.dismiss}
      title="An update is available"
      description={`Audexis ${state.version ?? ""}`}
      panelClassName="max-w-lg"
      bodyClassName="p-6 space-y-4"
      showCloseButton={!busy}
      closeOnEsc={!busy}
      closeOnOverlayClick={!busy}
      footer={
        <>
          <button
            disabled={busy}
            onClick={session.dismiss}
            className="rounded-md px-4 py-2 text-sm hover:bg-muted disabled:opacity-50"
          >
            Not now
          </button>
          <button
            disabled={busy}
            onClick={() => void session.install()}
            className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground disabled:opacity-50"
          >
            {state.phase === "installed"
              ? "Restart Audexis"
              : state.phase === "error"
                ? "Try again"
                : busy
                  ? "Updating…"
                  : "Update & restart"}
          </button>
        </>
      }
    >
      <p className="text-sm text-muted-foreground">
        Install the latest version and restart Audexis.
      </p>
      {state.notes && (
        <details className="text-sm">
          <summary className="cursor-pointer">What’s new</summary>
          <p className="mt-3 max-h-48 overflow-auto whitespace-pre-wrap text-muted-foreground">
            {state.notes}
          </p>
        </details>
      )}
      {busy && (
        <div role="status" className="space-y-2 text-sm">
          <p>
            {state.phase === "installing"
              ? "Installing update…"
              : `Downloading update${state.progress === undefined ? "…" : ` · ${state.progress}%`}`}
          </p>
          <progress
            className="w-full accent-primary"
            aria-label="Update download progress"
            max={100}
            value={state.progress}
          />
        </div>
      )}
      {state.error && (
        <p role="alert" className="text-sm text-destructive">
          {state.error}
        </p>
      )}
    </Modal>
  );
}
