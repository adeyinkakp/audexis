import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useQueryClient } from "@tanstack/react-query";
import { useStore } from "./useStore";
export function useSettingsDraft() {
  const { preferences, savePreferences } = useStore();
  const client = useQueryClient();
  const [appearance, setAppearance] = useState(preferences);
  const [folders, setFolders] = useState<string[]>([]);
  const originalFolders = useRef<string[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let disposed = false;
    void invoke<string[]>("get_library_roots")
      .then((values) => {
        if (disposed) return;
        originalFolders.current = values;
        setFolders(values);
        setLoaded(true);
      })
      .catch((error) => {
        if (!disposed) setError(String(error));
      });
    return () => {
      disposed = true;
    };
  }, []);
  const foldersChanged =
    JSON.stringify([...folders].sort()) !==
    JSON.stringify([...originalFolders.current].sort());
  const refresh = async () => {
    await Promise.all([
      client.invalidateQueries({ queryKey: ["fileWatcherMap"] }),
      client.invalidateQueries({ queryKey: ["playlist"] }),
      client.invalidateQueries({ queryKey: ["playlists"] }),
    ]);
  };
  const save = async (onSaved: () => void | Promise<void>) => {
    if (!loaded || busy) return;
    setBusy(true);
    setError(null);
    try {
      if (foldersChanged) {
        await invoke("set_library_roots", { folders });
        originalFolders.current = [...folders];
        await refresh();
      }
      await savePreferences(appearance);
      await onSaved();
    } catch (error) {
      setError(String(error));
    } finally {
      setBusy(false);
    }
  };
  const rescan = async () => {
    setBusy(true);
    setError(null);
    try {
      await invoke("rescan_library");
      await refresh();
    } catch (error) {
      setError(String(error));
    } finally {
      setBusy(false);
    }
  };
  return {
    appearance,
    setAppearance,
    folders,
    setFolders,
    foldersChanged,
    loaded,
    busy,
    error,
    save,
    rescan,
  };
}
