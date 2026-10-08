import { createContext } from "react";
import type { LazyStore } from "@tauri-apps/plugin-store";

export type SettingsTab =
  "appearance" | "library" | "metadata" | "customFields" | "logs";
export type ThemePreference = "light" | "dark" | "system";
export type RowDensity = "compact" | "default" | "comfort";
export type Preferences = {
  theme: ThemePreference;
  density: RowDensity;
  reduceMotion: boolean;
};
type StoreContextValue = {
  store: LazyStore;
  currentTheme: "light" | "dark";
  preferences: Preferences;
  savePreferences: (values: Preferences) => Promise<void>;
  openSettings: (tab?: SettingsTab) => void;
  openLibrarySettings: () => void;
  openTrackInfo: (fileIds: number[], initialTab?: "details" | "custom") => void;
  openLogs: () => void;
  openCustomFieldSettings: (initial?: {
    key: string;
    kind: "text" | "url";
  }) => void;
};
export const defaults: Preferences = {
  theme: "system",
  density: "default",
  reduceMotion: false,
};
export const StoreContext = createContext<StoreContextValue | null>(null);
