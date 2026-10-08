import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import type { MetadataFieldCatalog } from "../utils/metadataFields";

export function useMetadataFieldCatalog() {
  return useQuery({
    queryKey: ["metadataFieldCatalog"],
    queryFn: () => invoke<MetadataFieldCatalog>("get_metadata_field_catalog"),
    staleTime: Infinity,
  });
}
