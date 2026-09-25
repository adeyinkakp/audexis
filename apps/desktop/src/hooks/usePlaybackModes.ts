import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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

    void invoke<PlaybackModes>("get_playback_modes").then(setModes);
    void listen<PlaybackModes>("playback-modes-changed", (event) => {
      setModes(event.payload);
    }).then((unlisten) => {
      cleanup = unlisten;
    });

    return () => cleanup?.();
  }, []);

  return {
    modes,
    setModes,
  };
}
