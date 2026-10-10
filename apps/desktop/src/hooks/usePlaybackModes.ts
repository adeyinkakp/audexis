import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { logError } from "../utils/logger";
import { useEffect, useState } from "react";

export type RepeatMode = "off" | "queue" | "track";

export type PlaybackModes = {
  repeat_mode: RepeatMode;
  shuffled: boolean;
};

const DEFAULT_MODES: PlaybackModes = {
  repeat_mode: "off",
  shuffled: false,
};

export function usePlaybackModes() {
  const [modes, setModes] = useState<PlaybackModes>(DEFAULT_MODES);

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    let disposed = false;
    let receivedEvent = false;

    void listen<PlaybackModes>("playback-modes-changed", (event) => {
      receivedEvent = true;
      if (!disposed) setModes(event.payload);
    }).then(async (unlisten) => {
      if (disposed) { unlisten(); return; }
      cleanup = unlisten;
      const initial = await invoke<PlaybackModes>("get_playback_modes");
      if (!disposed && !receivedEvent) setModes(initial);
    }).catch((error) => logError("Load playback modes", error));

    return () => { disposed = true; cleanup?.(); };
  }, []);

  return {
    modes,
    setModes,
  };
}
