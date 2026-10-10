import type { CustomFieldDefinition } from "./customFields";

export type MetadataFieldInfo = {
  key: string;
  storageKey: string;
  label: string;
  kind: "text" | "url" | "custom" | "artwork" | "binary";
  multiValue: boolean;
  editable: boolean;
  nativeKey: string | null;
};
export type MetadataFieldCatalog = {
  fields: MetadataFieldInfo[];
  customKeys: string[];
};
export type RegularValues = Record<string, string[]>;
export type RegularChange =
  | { operation: "delete" }
  | { operation: "replace"; values: { type: "Text"; value: string }[] };

export function textValuesChange(values: string[]): RegularChange {
  const kept = values.filter((value) => value !== "");
  return kept.length
    ? {
        operation: "replace",
        values: kept.map((value) => ({ type: "Text", value })),
      }
    : { operation: "delete" };
}

export function regularFieldChanges(
  initial: RegularValues,
  draft: RegularValues,
): Record<string, RegularChange> {
  const changes: Record<string, RegularChange> = {};
  for (const key of new Set([...Object.keys(initial), ...Object.keys(draft)])) {
    const before = initial[key] ?? [];
    const after = (draft[key] ?? []).filter((value) => value !== "");
    if (JSON.stringify(before) === JSON.stringify(after)) continue;
    changes[key] = after.length
      ? {
          operation: "replace",
          values: after.map((value) => ({ type: "Text", value })),
        }
      : { operation: "delete" };
  }
  return changes;
}

export function regularValue(
  storageKey: string,
  metadata: { key: string; value: string; ord: number }[],
) {
  return metadata
    .filter((tag) => tag.key === storageKey)
    .sort((a, b) => a.ord - b.ord)
    .map((tag) => tag.value)
    .join("; ");
}

export function discoveredCustomFields(
  keys: string[],
  saved: CustomFieldDefinition[],
): CustomFieldDefinition[] {
  const fields = [...saved];
  for (const storageKey of keys) {
    const match = /^custom:(text|url):(.*)$/s.exec(storageKey);
    if (!match) continue;
    const kind = match[1] as "text" | "url";
    const key = match[2];
    if (
      fields.some(
        (field) =>
          field.key === key && (field.kind === kind || kind === "text"),
      )
    )
      continue;
    fields.push({
      id: `discovered-${encodeURIComponent(storageKey)}`,
      title: `${key} (${kind})`,
      key,
      kind,
    });
  }
  return fields;
}

export function sharedMetadataValues(
  files: {
    files: { id: number }[];
    metadata: { file_id: number; key: string; value: string; ord: number }[];
  },
  key: string,
) {
  const values = files.files.map((file) =>
    files.metadata
      .filter((item) => item.file_id === file.id && item.key === key)
      .sort((a, b) => a.ord - b.ord)
      .map((item) => item.value),
  );
  const first = values[0] ?? [];
  const mixed = values.some(
    (value) => JSON.stringify(value) !== JSON.stringify(first),
  );
  return { value: mixed ? [] : first, mixed };
}
