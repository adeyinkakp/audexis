import { Power, RotateCcw } from "lucide-react";
import { Modal } from "../components/Modal";
import {
  activeCurve,
  customizePreset,
  editBand,
} from "../components/equalizer/settings";
import type { useEqualizer } from "../hooks/useEqualizer";
import { cn } from "../utils";

function decibels(value: number) {
  return `${value > 0 ? "+" : ""}${value} dB`;
}
function frequencyLabel(value: number) {
  return value >= 1000 ? `${value / 1000}k` : String(value);
}

type Props = ReturnType<typeof useEqualizer> & {
  open: boolean;
  onClose: () => void;
};
export default function EqualizerModal({
  open,
  onClose,
  config,
  error,
  saving,
  update,
  reload,
}: Props) {
  const settings = config?.settings;
  const preset = config?.presets.find((item) => item.id === settings?.preset);
  const curve =
    config && settings ? activeCurve(settings, config.presets) : null;
  const manual = settings?.mode === "manual";
  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Equalizer"
      panelClassName="max-w-3xl"
      footer={
        <>
          <span role="status" className="mr-auto text-xs text-muted-foreground">
            {saving
              ? "Saving…"
              : error
                ? "Changes not saved"
                : config
                  ? "Settings saved automatically"
                  : "Loading…"}
          </span>
          <button
            type="button"
            onClick={onClose}
            className="rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:opacity-90"
          >
            Done
          </button>
        </>
      }
    >
      <div className="space-y-3 p-4">
        {error && (
          <div
            role="alert"
            className="flex items-center justify-between gap-4 rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm"
          >
            <span>
              {config
                ? "Could not apply and save your EQ changes."
                : "Could not load the equalizer."}{" "}
              <span className="block text-xs text-muted-foreground">
                {error}
              </span>
            </span>
            <button
              type="button"
              className="shrink-0 text-primary underline"
              onClick={() => (config ? update(config.settings) : void reload())}
            >
              Retry
            </button>
          </div>
        )}
        {!config || !settings || !curve || !preset ? (
          !error && (
            <p
              role="status"
              className="py-10 text-center text-sm text-muted-foreground"
            >
              Loading equalizer…
            </p>
          )
        ) : (
          <>
            <div className="flex flex-wrap items-center justify-between gap-4">
              <button
                type="button"
                role="switch"
                aria-label="Enable equalizer"
                aria-checked={settings.enabled}
                onClick={() =>
                  update({ ...settings, enabled: !settings.enabled })
                }
                className={cn(
                  "inline-flex items-center gap-2 rounded-full border border-border px-4 py-2 text-sm font-medium transition-colors focus-visible:outline-2 focus-visible:outline-primary",
                  settings.enabled
                    ? "border-primary/30 bg-primary/10 text-primary"
                    : "bg-muted text-muted-foreground",
                )}
              >
                <Power size={15} /> {settings.enabled ? "On" : "Off"}
              </button>
            </div>

            <div className="flex flex-wrap items-center justify-between gap-3">
              <div
                role="group"
                aria-label="Equalizer mode"
                className="inline-flex rounded-lg bg-muted p-1"
              >
                {(["preset", "manual"] as const).map((mode) => (
                  <button
                    key={mode}
                    type="button"
                    aria-pressed={settings.mode === mode}
                    onClick={() => update({ ...settings, mode })}
                    className={cn(
                      "rounded-md px-5 py-2 text-sm font-medium text-muted-foreground focus-visible:outline-2 focus-visible:outline-primary",
                      settings.mode === mode &&
                        "bg-background text-foreground shadow-sm",
                    )}
                  >
                    {mode === "preset" ? "Presets" : "Manual"}
                  </button>
                ))}
              </div>
              {manual ? (
                <button
                  type="button"
                  onClick={() =>
                    update({
                      ...settings,
                      manualGains: Array(10).fill(0),
                      manualPreamp: 0,
                    })
                  }
                  className="inline-flex items-center gap-2 rounded-lg px-3 py-2 text-xs text-muted-foreground hover:bg-muted hover:text-foreground"
                >
                  <RotateCcw size={14} /> Reset manual
                </button>
              ) : (
                <label className="flex items-center gap-2 text-xs text-muted-foreground">
                  Preset
                  <select
                    value={settings.preset}
                    onChange={(event) =>
                      update({ ...settings, preset: event.target.value })
                    }
                    className="min-w-40 rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground focus-visible:outline-2 focus-visible:outline-primary"
                  >
                    {config.presets.map((item) => (
                      <option key={item.id} value={item.id}>
                        {item.name}
                      </option>
                    ))}
                  </select>
                </label>
              )}
            </div>

            <div className="rounded-xl border border-border bg-muted/15 px-3 py-4 sm:px-5">
              <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
                <div>
                  <h3 className="text-sm font-medium">
                    {manual ? "Your custom sound" : preset.name}
                  </h3>
                  <p className="mt-1 text-xs text-muted-foreground"></p>
                </div>
                {!manual && (
                  <button
                    type="button"
                    onClick={() => update(customizePreset(settings, preset))}
                    className="rounded-lg border border-border bg-background px-3 py-2 text-xs font-medium hover:border-primary/50"
                  >
                    Customize this preset
                  </button>
                )}
              </div>
              <div className="overflow-x-auto pb-2">
                <div className="relative grid min-w-110 grid-cols-10 gap-1">
                  <div
                    aria-hidden="true"
                    className="pointer-events-none absolute inset-x-2 top-23 border-t border-dashed border-border"
                  />
                  {config.frequencies.map((frequency, index) => (
                    <label
                      key={frequency}
                      className="relative flex flex-col items-center gap-3"
                    >
                      <span className="text-[10px] tabular-nums text-muted-foreground">
                        {decibels(curve.gains[index])}
                      </span>
                      <input
                        type="range"
                        min={-12}
                        max={12}
                        step={0.5}
                        value={curve.gains[index]}
                        disabled={!manual}
                        aria-label={`${frequency} Hz gain`}
                        aria-valuetext={decibels(curve.gains[index])}
                        onChange={(event) =>
                          update(
                            editBand(
                              settings,
                              index,
                              Number(event.target.value),
                            ),
                          )
                        }
                        style={{ writingMode: "vertical-lr", direction: "rtl" }}
                        className="h-32 w-7 cursor-pointer accent-primary disabled:cursor-default disabled:opacity-60 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-primary"
                      />
                      <span className="text-xs font-medium tabular-nums">
                        {frequencyLabel(frequency)}
                      </span>
                    </label>
                  ))}
                </div>
              </div>
              <div
                aria-hidden="true"
                className="mt-3 flex justify-between text-[10px] uppercase tracking-widest text-muted-foreground"
              >
                <span>Bass</span>
                <span>Midrange</span>
                <span>Treble</span>
              </div>
            </div>

            <div className="grid gap-3 sm:grid-cols-[1fr_1fr] sm:items-center sm:gap-8">
              <div>
                <label htmlFor="eq-preamp" className="text-sm font-medium">
                  Preamp
                </label>
                <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                  Lower the overall level if boosted bands sound distorted.
                </p>
              </div>
              <div className="flex items-center gap-3">
                <input
                  id="eq-preamp"
                  type="range"
                  min={-24}
                  max={0}
                  step={0.5}
                  disabled={!manual}
                  value={curve.preamp}
                  aria-valuetext={decibels(curve.preamp)}
                  onChange={(event) =>
                    update({
                      ...settings,
                      manualPreamp: Number(event.target.value),
                    })
                  }
                  className="h-6 min-w-0 flex-1 cursor-pointer accent-primary disabled:cursor-default disabled:opacity-60"
                />
                <output
                  htmlFor="eq-preamp"
                  className="w-16 text-right text-xs tabular-nums text-muted-foreground"
                >
                  {decibels(curve.preamp)}
                </output>
              </div>
            </div>
          </>
        )}
      </div>
    </Modal>
  );
}
