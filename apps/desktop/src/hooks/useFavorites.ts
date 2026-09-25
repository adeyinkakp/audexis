import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import toast from "react-hot-toast";
export function useFavorites() {
  const client = useQueryClient();
  const favorites = useQuery({
    queryKey: ["favoriteIds"],
    queryFn: () => invoke<number[]>("get_favorite_ids"),
    staleTime: Infinity,
  });
  const mutation = useMutation({
    mutationKey: ["setMediaLoved"],
    mutationFn: ({ fileId, loved }: { fileId: number; loved: boolean }) =>
      invoke("set_media_loved", { fileId, loved }),
    onSuccess: (_data, { fileId, loved }) => {
      void client.invalidateQueries({ queryKey: ["homeDiscovery"] });
      client.setQueryData<number[]>(["favoriteIds"], (old) =>
        loved
          ? [...new Set([...(old ?? []), fileId])]
          : (old ?? []).filter((id) => id !== fileId),
      );
      void client.invalidateQueries({
        queryKey: ["searchMedia"],
        predicate: (query) =>
          typeof query.queryKey[1] === "string" &&
          query.queryKey[1].includes('"favoritesOnly":true'),
      });
    },
    onError: (error) => toast.error(String(error)),
  });
  return { ...favorites, setLoved: mutation.mutate };
}
