import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  createSettingsWriter,
  type EqualizerConfiguration,
  type EqualizerSettings,
} from "../components/equalizer/settings";

export function useEqualizer() {
  const [config, setConfig] = useState<EqualizerConfiguration | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const mounted = useRef(false);
  const load = useCallback(async () => {
    setError(null);
    try {
      const result = await invoke<EqualizerConfiguration>("get_equalizer");
      if (mounted.current) setConfig(result);
    } catch (cause) {
      if (mounted.current) setError(String(cause));
    }
  }, []);
  useEffect(() => {
    mounted.current = true;
    void load();
    return () => { mounted.current = false; };
  }, [load]);

  const write = useMemo(() => createSettingsWriter<EqualizerSettings>(
    (settings) => invoke("set_equalizer", { settings }),
    (cause, pending) => {
      if (!mounted.current) return;
      setSaving(pending);
      setError(cause == null ? null : String(cause));
    },
  ), []);
  const update = useCallback((settings: EqualizerSettings) => {
    setConfig((current) => current ? { ...current, settings } : current);
    write(settings);
  }, [write]);
  return { config, error, saving, update, reload: load };
}
