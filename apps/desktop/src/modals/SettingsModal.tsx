import { useState } from "react";
import {
  FileWarning,
  Folder,
  Palette,
  RefreshCw,
  Settings2,
  X,
} from "lucide-react";
import { Modal } from "../components/Modal";
import { AppearanceOptions } from "../components/settings/AppearanceOptions";
import { LibraryFolders } from "../components/settings/LibraryFolders";
import { useSettingsDraft } from "../hooks/useSettingsDraft";
import { cn } from "../utils";
export function SettingsModal({
  open,
  onClose,
  onOpenLogs,
}: {
  open: boolean;
  onClose: () => void;
  onOpenLogs: () => void;
}) {
  const [tab, setTab] = useState<"appearance" | "library" | "logs">(
    "appearance",
  );
  const draft = useSettingsDraft();
  const close = () => {
    if (!draft.busy) onClose();
  };
  return (
    <Modal
      open={open}
      onClose={close}
      title="Settings"
      header={null}
      closeOnOverlayClick={false}
      closeOnEsc={!draft.busy}
      panelClassName="max-w-[800px] rounded-3xl bg-background"
      bodyClassName="p-0"
    >
      <div className="flex items-center justify-between border-b border-border px-7 py-5">
        <div className="flex items-center gap-2.5">
          <Settings2 size={20} className="text-primary" />
          <h1 className="text-lg font-semibold">Settings</h1>
        </div>
        <button
          type="button"
          disabled={draft.busy}
          onClick={close}
          aria-label="Close settings"
          className="rounded-full p-2 text-muted-foreground hover:bg-muted"
        >
          <X size={18} />
        </button>
      </div>
      <div className="grid sm:grid-cols-[180px_minmax(0,1fr)]">
        <nav
          aria-label="Settings sections"
          className="flex gap-2 border-b border-border bg-muted/15 p-4 sm:flex-col sm:border-b-0 sm:border-r"
        >
          {(
            [
              { id: "appearance", label: "Appearance", Icon: Palette },
              { id: "library", label: "Library", Icon: Folder },
              { id: "logs", label: "Logs", Icon: FileWarning },
            ] as const
          ).map(({ id, label, Icon }) => (
            <button
              key={id}
              type="button"
              onClick={() => setTab(id)}
              aria-current={tab === id ? "page" : undefined}
              className={cn(
                "flex items-center gap-2 rounded-xl px-3 py-2.5 text-left text-sm",
                tab === id
                  ? "bg-primary/10 font-medium text-primary"
                  : "text-muted-foreground hover:bg-muted/50",
              )}
            >
              <Icon size={16} />
              {label}
            </button>
          ))}
        </nav>
        <section className="min-h-108 min-w-0 p-6 sm:p-8">
          <h2 className="text-xl font-semibold tracking-tight">
            {tab === "appearance"
              ? "Make yourself at home"
              : tab === "library"
                ? "Your music"
                : "Logs n shi"}
          </h2>
          <p className="mb-6 mt-2 text-sm text-muted-foreground">
            {tab === "appearance"
              ? "Change how Audexis looks."
              : tab === "library"
                ? "Manage the folders Audexis watches for music."
                : "Review backend errors and logs."}
          </p>
          <fieldset
            disabled={tab !== "logs" && (draft.busy || !draft.loaded)}
            className="min-w-0 disabled:opacity-60"
          >
            {tab === "appearance" ? (
              <AppearanceOptions
                value={draft.appearance}
                onChange={draft.setAppearance}
              />
            ) : tab === "library" ? (
              <div className="space-y-5">
                <LibraryFolders
                  folders={draft.folders}
                  onChange={draft.setFolders}
                  disabled={!draft.loaded || draft.busy}
                />
                <p className="text-xs leading-relaxed text-muted-foreground">
                  Removing a folder removes its songs from the library and
                  playlists when you save. Files aren't removed from disk
                </p>
                <div className="flex items-center justify-between gap-4 border-t border-border pt-5">
                  <div>
                    <p className="text-sm font-medium">Refresh library</p>
                    <p className="mt-1 text-xs text-muted-foreground">
                      Scan saved folders for changes and missing metadata.
                    </p>
                  </div>
                  <button
                    type="button"
                    disabled={
                      draft.foldersChanged ||
                      draft.folders.length === 0 ||
                      draft.busy
                    }
                    onClick={() => void draft.rescan()}
                    className="inline-flex shrink-0 items-center gap-2 rounded-lg border border-border px-3 py-2 text-xs hover:bg-muted disabled:opacity-40"
                  >
                    <RefreshCw size={13} />
                    Rescan
                  </button>
                </div>
                {draft.foldersChanged && (
                  <p className="text-xs text-muted-foreground">
                    Save your folder changes before rescanning.
                  </p>
                )}
              </div>
            ) : (
              <div className="rounded-2xl border border-border bg-muted/20 p-5">
                <div className="flex items-start gap-3">
                  <FileWarning
                    size={18}
                    className="mt-0.5 shrink-0 text-primary"
                  />
                  <div>
                    <p className="text-sm font-medium">Backend error logs</p>
                    <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                      View errors recorded by the database, metadata backend,
                      audio player, and file watcher.
                    </p>
                    <button
                      type="button"
                      onClick={onOpenLogs}
                      className="mt-4 rounded-full bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground hover:opacity-90"
                    >
                      View logs
                    </button>
                  </div>
                </div>
              </div>
            )}
          </fieldset>
          {tab !== "logs" && !draft.loaded && !draft.error && (
            <p className="mt-4 text-xs text-muted-foreground">
              Loading preferences…
            </p>
          )}
          {tab !== "logs" && draft.error && (
            <p
              role="alert"
              className="mt-4 rounded-xl bg-destructive/10 p-3 text-sm text-destructive"
            >
              {draft.error}
            </p>
          )}
        </section>
      </div>
      <div className="flex items-center justify-between gap-4 border-t border-border px-7 py-5">
        <p className="text-xs text-muted-foreground">
          {tab === "logs"
            ? "Logs are saved automatically."
            : draft.busy
              ? "Updating your preferences…"
              : "Changes apply when you save."}
        </p>
        <div className="flex gap-3">
          <button
            type="button"
            disabled={draft.busy}
            onClick={close}
            className="rounded-full px-4 py-2 text-sm text-muted-foreground hover:bg-muted"
          >
            {tab === "logs" ? "Close" : "Cancel"}
          </button>
          {tab !== "logs" && (
            <button
              type="button"
              disabled={draft.busy || !draft.loaded}
              onClick={() => void draft.save(onClose)}
              className="rounded-full bg-primary px-5 py-2 text-sm font-semibold text-primary-foreground hover:opacity-90 disabled:opacity-50"
            >
              {draft.busy ? "Saving…" : "Save changes"}
            </button>
          )}
        </div>
      </div>
    </Modal>
  );
}
