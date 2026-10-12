import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import { artworkCachePolicy } from "../utils/queryCachePolicy";

export function useArtwork(fileId: number | null) {
  return useQuery({
    queryKey: ["libraryArtwork", fileId],
    queryFn: async ({ signal }) => {
      signal.throwIfAborted();
      const artwork = await invoke<string | null>("get_artwork", { fileId });
      signal.throwIfAborted();
      return artwork;
    },
    enabled: fileId !== null && fileId > 0,
    ...artworkCachePolicy,
  });
}
