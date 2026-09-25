import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { LazyStore } from "@tauri-apps/plugin-store";
import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import toast from "react-hot-toast";
import { OnboardingModal } from "../modals/OnboardingModal";
import { SettingsModal } from "../modals/SettingsModal";
import { LogsModal } from "../modals/LogsModal";
import { logError } from "../utils/logger";

const store = new LazyStore("./settings.json");
export type ThemePreference = "light" | "dark" | "system";
export type RowDensity = "compact" | "default" | "comfort";
type Preferences = {
  theme: ThemePreference;
  density: RowDensity;
  reduceMotion: boolean;
};
type BackendErrorEvent = {
  kind: string;
  message: string;
  details: string;
};
type StoreContextValue = {
  store: LazyStore;
  currentTheme: "light" | "dark";
  preferences: Preferences;
  savePreferences: (values: Preferences) => Promise<void>;
  openSettings: () => void;
  openLogs: () => void;
};
const defaults: Preferences = {
  theme: "system",
  density: "default",
  reduceMotion: false,
};
const StoreContext = createContext<StoreContextValue | null>(null);
export function useStore() {
  const context = useContext(StoreContext);
  if (!context) throw new Error("useStore must be used within StoreProvider");
  return context;
}
export function StoreProvider({ children }: { children: ReactNode }) {
  const [preferences, setPreferences] = useState<Preferences>(defaults);
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia("(prefers-color-scheme: dark)").matches,
  );
  const [needsOnboarding, setNeedsOnboarding] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [logsOpen, setLogsOpen] = useState(false);
  const currentTheme =
    preferences.theme === "system"
      ? systemDark
        ? "dark"
        : "light"
      : preferences.theme;
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<BackendErrorEvent>("backend-error", ({ payload }) => {
      const id = toast.custom(
        (current) => (
          <button
            type="button"
            onClick={() => {
              toast.dismiss(current.id);
              setLogsOpen(true);
            }}
            className="max-w-md rounded-xl border border-destructive/30 bg-background px-4 py-3 text-left shadow-lg"
          >
            <span className="block text-sm font-semibold text-destructive">
              {payload.kind}
            </span>
            <span className="mt-1 block text-sm text-foreground">
              {payload.message}
            </span>
            <span className="mt-2 block text-xs text-muted-foreground">
              Click to view logs
            </span>
          </button>
        ),
        { duration: 8000 },
      );
      return id;
    }).then((cleanup) => {
      if (disposed) cleanup();
      else unlisten = cleanup;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const change = () => setSystemDark(media.matches);
    media.addEventListener("change", change);
    return () => media.removeEventListener("change", change);
  }, []);
  useEffect(() => {
    document.documentElement.dataset.theme = currentTheme;
    document.documentElement.dataset.density = preferences.density;
    document.documentElement.dataset.reduceMotion = String(
      preferences.reduceMotion,
    );
    localStorage.setItem("theme", currentTheme);
  }, [currentTheme, preferences]);
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const read = async () => {
      const [theme, density, reduceMotion] = await Promise.all([
        store.get("theme"),
        store.get("density"),
        store.get("reduceMotion"),
      ]);
      if (!disposed)
        setPreferences({
          theme: theme === "light" || theme === "dark" ? theme : "system",
          density:
            density === "compact" || density === "comfort"
              ? density
              : "default",
          reduceMotion: reduceMotion === true,
        });
    };
    void (async () => {
      await store.init();
      const cleanup = await store.onChange((key) => {
        if (["theme", "density", "reduceMotion"].includes(key)) void read();
      });
      if (disposed) {
        cleanup();
        return;
      }
      unlisten = cleanup;
      await read();
      const completed = await store.get("needsOnboarding");
      if (!disposed) setNeedsOnboarding(completed !== false);
    })()
      .catch((error) => {
        logError("Could not load preferences", error);
        if (!disposed) toast.error("Could not load preferences");
      })
      .finally(() => {
        if (!disposed) void getCurrentWindow().show().catch(console.error);
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
  useEffect(() => {
    const keydown = (event: KeyboardEvent) => {
      if (
        (event.metaKey || event.ctrlKey) &&
        event.key === "," &&
        !needsOnboarding
      ) {
        event.preventDefault();
        setSettingsOpen(true);
      }
    };
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  }, [needsOnboarding]);
  const savePreferences = async (values: Preferences) => {
    await store.set("theme", values.theme);
    await store.set("density", values.density);
    await store.set("reduceMotion", values.reduceMotion);
    await store.save();
    setPreferences(values);
  };
  const completeOnboarding = async () => {
    await store.set("needsOnboarding", false);
    await store.save();
    setNeedsOnboarding(false);
  };
  return (
    <StoreContext.Provider
      value={{
        store,
        currentTheme,
        preferences,
        savePreferences,
        openSettings: () => setSettingsOpen(true),
        openLogs: () => setLogsOpen(true),
      }}
    >
      <div className="h-full w-full">
        {children}
        {needsOnboarding && (
          <OnboardingModal open onClose={completeOnboarding} />
        )}
        {settingsOpen && (
          <SettingsModal
            open
            onClose={() => setSettingsOpen(false)}
            onOpenLogs={() => {
              setSettingsOpen(false);
              setLogsOpen(true);
            }}
          />
        )}
        {logsOpen && <LogsModal open onClose={() => setLogsOpen(false)} />}
      </div>
    </StoreContext.Provider>
  );
}
