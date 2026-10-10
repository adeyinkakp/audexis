export type EqualizerSettings = {
  enabled: boolean;
  mode: "preset" | "manual";
  preset: string;
  manualGains: number[];
  manualPreamp: number;
};
export type EqualizerPreset = {
  id: string;
  name: string;

  gains: number[];
  preamp: number;
};
export type EqualizerConfiguration = {
  settings: EqualizerSettings;
  presets: EqualizerPreset[];
  frequencies: number[];
};

export function activeCurve(
  settings: EqualizerSettings,
  presets: EqualizerPreset[],
) {
  const preset =
    presets.find((item) => item.id === settings.preset) ?? presets[0];
  return settings.mode === "manual"
    ? { gains: settings.manualGains, preamp: settings.manualPreamp }
    : { gains: preset.gains, preamp: preset.preamp };
}

export function customizePreset(
  settings: EqualizerSettings,
  preset: EqualizerPreset,
): EqualizerSettings {
  return {
    ...settings,
    mode: "manual",
    manualGains: [...preset.gains],
    manualPreamp: preset.preamp,
  };
}

export function editBand(
  settings: EqualizerSettings,
  index: number,
  gain: number,
): EqualizerSettings {
  return {
    ...settings,
    mode: "manual",
    manualGains: settings.manualGains.map((value, i) =>
      i === index ? gain : value,
    ),
  };
}

export function createSettingsWriter<T>(
  write: (value: T) => Promise<void>,
  onStatus: (error: unknown | null, saving: boolean) => void,
) {
  let pending: { value: T } | undefined;
  let running = false;
  async function drain() {
    if (running) return;
    running = true;
    while (pending) {
      const next = pending.value;
      pending = undefined;
      try {
        await write(next);
        if (!pending) onStatus(null, false);
      } catch (error) {
        if (!pending) onStatus(error, false);
      }
    }
    running = false;
  }
  return (value: T) => {
    pending = { value };
    onStatus(null, true);
    void drain();
  };
}
