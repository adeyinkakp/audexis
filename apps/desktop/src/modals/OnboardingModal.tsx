import { useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  Folder,
  Headphones,
  Library,
  Music2,
  Palette,
} from "lucide-react";
import { Modal } from "../components/Modal";
import { AppearanceOptions } from "../components/settings/AppearanceOptions";
import { LibraryFolders } from "../components/settings/LibraryFolders";
import { useSettingsDraft } from "../hooks/useSettingsDraft";
import { cn } from "../utils";
const steps = [
  { label: "Your library", Icon: Folder },
  { label: "Make it yours", Icon: Palette },
  { label: "Ready to listen", Icon: Headphones },
];
export function OnboardingModal({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void | Promise<void>;
}) {
  const [step, setStep] = useState(0);
  const draft = useSettingsDraft();
  return (
    <Modal
      open={open}
      onClose={() => {}}
      title="Welcome to Audexis"
      header={null}
      showCloseButton={false}
      closeOnEsc={false}
      closeOnOverlayClick={false}
      panelClassName="max-w-215 rounded-3xl bg-background shadow-2xl"
      bodyClassName="p-0"
    >
      <div className="]">
        <section className="flex min-h-132.5 flex-col p-6 sm:p-9">
          <p className="mb-2 text-[10px] font-semibold uppercase tracking-[0.2em] text-primary">
            Step {step + 1} of {steps.length}
          </p>

          <p className="mb-7 mt-3 text-sm leading-relaxed text-muted-foreground">
            {
              [
                "Choose the folders you listen from. We’ll find your songs and read their tags, artwork, and durations.",
                "Choose an appearance and the amount of space that feels right. You can change these later in Settings.",
                "Everything is set. Your library will keep up as files change, and you can start making playlists right away.",
              ][step]
            }
          </p>
          <fieldset
            disabled={draft.busy || !draft.loaded}
            className="min-w-0 flex-1 disabled:opacity-60"
          >
            {step === 0 && (
              <>
                {!draft.loaded && !draft.error ? (
                  <p className="text-sm text-muted-foreground">
                    Loading library settings…
                  </p>
                ) : (
                  <LibraryFolders
                    folders={draft.folders}
                    onChange={draft.setFolders}
                    disabled={!draft.loaded}
                  />
                )}
                <p className="mt-5 text-xs text-muted-foreground">
                  You can also start with an empty library and add folders
                  later.
                </p>
              </>
            )}
            {step === 1 && (
              <AppearanceOptions
                value={draft.appearance}
                onChange={draft.setAppearance}
              />
            )}
            {step === 2 && (
              <div className="space-y-4">
                <div className="rounded-2xl border border-border bg-muted/20 p-5">
                  <div className="flex items-center gap-3">
                    <Library size={20} className="text-primary" />
                    <div>
                      <p className="text-sm font-semibold">
                        {draft.folders.length
                          ? `${draft.folders.length} music ${draft.folders.length === 1 ? "folder" : "folders"}`
                          : "A fresh start"}
                      </p>
                      <p className="mt-1 text-xs text-muted-foreground">
                        {draft.folders.length
                          ? "Subfolders included"
                          : "Add your music from Settings whenever you’re ready"}
                      </p>
                    </div>
                  </div>
                  <div className="mt-4 border-t border-border pt-4 text-xs capitalize text-muted-foreground">
                    {draft.appearance.theme} appearance ·{" "}
                    {draft.appearance.density === "default"
                      ? "Balanced"
                      : draft.appearance.density}{" "}
                    rows
                  </div>
                </div>
                <ul className="space-y-3 text-sm text-muted-foreground">
                  <li className="flex gap-2">
                    <Check size={16} className="shrink-0 text-primary" />
                    Open Settings anytime with ⌘, or Ctrl+,.
                  </li>
                </ul>
              </div>
            )}
          </fieldset>
          {draft.error && (
            <p
              role="alert"
              className="mt-4 rounded-xl bg-destructive/10 p-3 text-sm text-destructive"
            >
              {draft.error}
            </p>
          )}
          <div className="mt-8 flex items-center justify-between gap-3 border-t border-border/60 pt-5">
            <button
              type="button"
              disabled={step === 0 || draft.busy}
              onClick={() => setStep(step - 1)}
              className="inline-flex items-center gap-1 text-sm text-muted-foreground disabled:invisible"
            >
              <ArrowLeft size={15} />
              Back
            </button>
            <button
              type="button"
              disabled={!draft.loaded || draft.busy}
              onClick={() =>
                step < 2 ? setStep(step + 1) : void draft.save(onClose)
              }
              className="inline-flex items-center gap-2 rounded-full bg-primary px-5 py-2.5 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-50"
            >
              {draft.busy
                ? "Setting up your library…"
                : step === 2
                  ? "Start listening"
                  : step === 0 && draft.folders.length === 0
                    ? "Add music later"
                    : "Continue"}
              {!draft.busy && <ArrowRight size={16} />}
            </button>
          </div>
        </section>
      </div>
    </Modal>
  );
}
