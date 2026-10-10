import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useStore } from "./useStore";
import {
  normalizeDefinitions,
  validateDefinitions,
  type CustomFieldDefinition,
} from "../utils/customFields";

const KEY = "custom-field-definitions";
const queryKey = ["customFieldDefinitions"];
let writes = Promise.resolve();
const EMPTY: CustomFieldDefinition[] = [];

export function useCustomFieldDefinitions() {
  const { store } = useStore();
  const client = useQueryClient();
  const query = useQuery({
    queryKey,
    queryFn: async () => {
      await writes;
      return normalizeDefinitions(await store.get(KEY));
    },
    staleTime: Infinity,
  });
  const mutation = useMutation({
    mutationFn: async (definitions: CustomFieldDefinition[]) => {
      const error = validateDefinitions(definitions);
      if (error) throw new Error(error);
      const next = writes.then(async () => {
        await store.set(KEY, definitions);
        await store.save();
      });
      writes = next.catch(() => {});
      await next;
      return definitions;
    },
    onSuccess: (definitions) => client.setQueryData(queryKey, definitions),
  });
  return {
    ...query,
    definitions: query.data ?? EMPTY,
    save: mutation.mutateAsync,
    saving: mutation.isPending,
  };
}
