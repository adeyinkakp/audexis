export type DownloadEvent =
  | { event: "Started"; data: { contentLength?: number } }
  | { event: "Progress"; data: { chunkLength: number } }
  | { event: "Finished" };

export interface AvailableUpdate {
  version: string;
  body?: string;
  downloadAndInstall: (
    onEvent: (event: DownloadEvent) => void,
  ) => Promise<void>;
  close: () => Promise<void>;
}

export interface UpdateState {
  phase:
    | "idle"
    | "available"
    | "downloading"
    | "installing"
    | "installed"
    | "error"
    | "dismissed";
  version?: string;
  notes?: string;
  progress?: number;
  error?: string;
}

export function createUpdateSession(deps: {
  check: () => Promise<AvailableUpdate | null>;
  restart: () => Promise<void>;
  log: (context: string, error: unknown) => void;
}) {
  let checked = false;
  let busy = false;
  let update: AvailableUpdate | null = null;
  let state: UpdateState = { phase: "idle" };
  const listeners = new Set<() => void>();
  const publish = (change: Partial<UpdateState>) => {
    state = { ...state, ...change };
    listeners.forEach((listener) => listener());
  };
  const restart = async () => {
    try {
      await deps.restart();
    } catch (error) {
      deps.log("Update restart failed", error);
      publish({
        error: "The update is installed. Close and reopen Audexis to finish.",
      });
    }
  };

  return {
    getSnapshot: () => state,
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    async check() {
      if (checked) return;
      checked = true;
      try {
        update = await deps.check();
        if (update)
          publish({
            phase: "available",
            version: update.version,
            notes: update.body,
          });
      } catch (error) {
        deps.log("Startup update check failed", error);
      }
    },
    dismiss() {
      if (busy) return;
      publish({ phase: "dismissed" });
      const resource = update;
      update = null;
      void resource
        ?.close()
        .catch((error) => deps.log("Update cleanup failed", error));
    },
    async install() {
      if (
        busy ||
        !update ||
        !["available", "error", "installed"].includes(state.phase)
      )
        return;
      busy = true;
      if (state.phase === "installed") {
        await restart();
        busy = false;
        return;
      }
      publish({ phase: "downloading", error: undefined, progress: undefined });
      let downloaded = 0;
      let total: number | undefined;
      try {
        await update.downloadAndInstall((event) => {
          if (event.event === "Started") total = event.data.contentLength;
          if (event.event === "Progress") downloaded += event.data.chunkLength;
          if (event.event === "Finished") {
            publish({ phase: "installing", progress: 100 });
          } else {
            publish({
              progress: total
                ? Math.min(100, Math.round((downloaded / total) * 100))
                : undefined,
            });
          }
        });
        publish({ phase: "installed" });
        await restart();
      } catch (error) {
        deps.log("Update installation failed", error);
        publish({
          phase: "error",
          error:
            "The update couldn’t be installed. Try again, or keep using this version for now.",
        });
      } finally {
        busy = false;
      }
    },
  };
}
