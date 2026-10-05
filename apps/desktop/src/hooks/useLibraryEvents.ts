import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { subscribeLibraryChanges, libraryChangeIds } from "./libraryEvents";

export function useLibraryEvents() {
  const client = useQueryClient();
  useEffect(
    () =>
      subscribeLibraryChanges(
        client,
        (onChange) =>
          listen<{ file_ids?: number[]; fileIds?: number[] }>(
            "library-changed",
            ({ payload }) => onChange(libraryChangeIds(payload)),
          ),
        (error) => console.error("Could not refresh library data", error),
      ),
    [client],
  );
}
