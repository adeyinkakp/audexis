import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useStore } from "./useStore";

export const defaultDetailsFields = [
  "title",
  "artist",
  "album",
  "albumArtist",
  "genre",
  "year",
  "trackNumber",
  "discNumber",
  "composer",
  "comments",
];
const queryKey = ["detailsFields"];
let writes = Promise.resolve();

export function useDetailsFields() {
  const { store } = useStore();
  const client = useQueryClient();
  const query = useQuery({
    queryKey,
    queryFn: async () => {
      await writes;
      const saved = await store.get<unknown>("metadata-editor-fields");
      return Array.isArray(saved)
        ? [
            ...new Set(
              saved.filter((key): key is string => typeof key === "string"),
            ),
          ]
        : defaultDetailsFields;
    },
    staleTime: Infinity,
  });
  const mutation = useMutation({
    mutationFn: async (fields: string[]) => {
      const next = writes.then(async () => {
        await store.set("metadata-editor-fields", fields);
        await store.save();
      });
      writes = next.catch(() => {});
      await next;
      return fields;
    },
    onSuccess: (fields) => client.setQueryData(queryKey, fields),
  });
  return {
    ...query,
    fields: query.data ?? defaultDetailsFields,
    save: mutation.mutateAsync,
    saving: mutation.isPending,
  };
}
