import { useSyncExternalStore } from "react";

export const isMac = navigator.userAgent.includes("Mac");
export type AppMenuAction =
  | "settings"
  | "equalizer"
  | "mini-player"
  | "expanded"
  | "queue"
  | "lyrics"
  | "play"
  | "pause"
  | "previous"
  | "next"
  | "shuffle"
  | "repeat-off"
  | "repeat-queue"
  | "repeat-track";
export const APP_MENU_EVENT = "audexis-menu-action";
export function dispatchMenuAction(action: AppMenuAction) {
  window.dispatchEvent(new CustomEvent(APP_MENU_EVENT, { detail: action }));
}
export function listenMenuAction(handler: (action: AppMenuAction) => void) {
  const listener = (event: Event) =>
    handler((event as CustomEvent<AppMenuAction>).detail);
  window.addEventListener(APP_MENU_EVENT, listener);
  return () => window.removeEventListener(APP_MENU_EVENT, listener);
}

export type PlayerMenuState = {
  fileId: number;
  paused: boolean;
  shuffled: boolean;
  repeat: "off" | "queue" | "track";
  panel: "queue" | "lyrics" | null;
};
let playerState: PlayerMenuState = {
  fileId: 0,
  paused: false,
  shuffled: false,
  repeat: "off",
  panel: null,
};
const subscribers = new Set<() => void>();
export function publishPlayerMenuState(state: PlayerMenuState) {
  playerState = state;
  subscribers.forEach((notify) => notify());
}
export function usePlayerMenuState() {
  return useSyncExternalStore(
    (notify) => {
      subscribers.add(notify);
      return () => {
        subscribers.delete(notify);
      };
    },
    () => playerState,
  );
}
