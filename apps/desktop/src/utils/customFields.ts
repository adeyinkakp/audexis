import type { MetadataFieldInfo, RegularValues } from "./metadataFields";

export type CustomFields = {
  supported: boolean;
  supportsUrls: boolean;
  format: string;
  regular: RegularValues;
  fields: MetadataFieldInfo[];
  text: { description: string; value: string }[];
  urls: { description: string; url: string }[];
};
export type CustomField = {
  id: string;
  kind: "text" | "url";
  name: string;
  value: string;
};

export function customFieldRows(
  fields: CustomFields,
  definitions: CustomFieldDefinition[] = [],
): CustomField[] {
  return [
    ...fields.text.map((entry, index) => ({
      id: `text-${index}`,
      kind:
        !fields.supportsUrls &&
        definitions.some(
          (field) =>
            field.kind === "url" &&
            fieldStorageKey(field, fields.format) ===
              fieldStorageKey(
                { kind: "text", key: entry.description },
                fields.format,
              ),
        )
          ? ("url" as const)
          : ("text" as const),
      name: entry.description,
      value: entry.value,
    })),
    ...fields.urls.map((entry, index) => ({
      id: `url-${index}`,
      kind: "url" as const,
      name: entry.description,
      value: entry.url,
    })),
  ];
}

export function validateCustomFields(
  rows: CustomField[],
  _format = "id3v2.3",
): string | null {
  const names = new Set<string>();
  for (const row of rows) {
    if (row.name.includes("="))
      return "Metadata keys cannot contain an equals sign (=).";
    if (row.name.includes("\0") || row.value.includes("\0"))
      return "Custom fields cannot contain NUL characters.";
    if (!row.name || /[^\x20-\x7d]|=/.test(row.name))
      return "FLAC/Ogg keys must be without '='.";
    if (!row.name) return "fields need a metadata key.";
    const key = JSON.stringify([row.kind, row.name]);
    if (names.has(key))
      return "Fields of the same type must have unique names.";
    names.add(key);
    if (row.kind === "url" && /[^\x00-\x7f]/.test(row.value))
      return "Use a percent-encoded URL for custom URL fields.";
  }
  return null;
}

export function customFieldChanges(rows: CustomField[], supportsUrls = true) {
  const text = rows
    .filter((row) => row.kind === "text" || !supportsUrls)
    .map((row) => ({
      type: "UserText" as const,
      value: { description: row.name, value: row.value },
    }));
  const urls = rows
    .filter((row) => row.kind === "url")
    .map((row) => ({
      type: "UserUrl" as const,
      value: { description: row.name, url: row.value },
    }));
  return {
    userDefinedText: { operation: "replace" as const, values: text },
    ...(supportsUrls
      ? { userDefinedUrl: { operation: "replace" as const, values: urls } }
      : {}),
  };
}

export type CustomFieldDefinition = {
  id: string;
  title: string;
  key: string;
  kind: "text" | "url";
};

export function validateDefinitions(
  definitions: CustomFieldDefinition[],
): string | null {
  const identities = new Set<string>();
  const ids = new Set<string>();
  for (const field of definitions) {
    if (!field.title.trim() || !field.key.trim())
      return "Each field needs a display title and metadata key.";
    if (field.key.includes("="))
      return "Metadata keys cannot contain an equals sign (=).";
    if (field.key.includes("\0"))
      return "Metadata keys cannot contain NUL characters.";
    const identity = JSON.stringify([field.kind, field.key]);
    if (identities.has(identity))
      return "This metadata key is already saved for that field type.";
    if (!field.id || ids.has(field.id)) return "Field IDs must be unique.";
    identities.add(identity);
    ids.add(field.id);
  }
  return null;
}

export function normalizeDefinitions(value: unknown): CustomFieldDefinition[] {
  if (!Array.isArray(value)) return [];
  return value.filter(
    (field): field is CustomFieldDefinition =>
      !!field &&
      typeof field === "object" &&
      typeof field.id === "string" &&
      typeof field.title === "string" &&
      typeof field.key === "string" &&
      (field.kind === "text" || field.kind === "url"),
  );
}

export function fieldStorageKey(
  field: Pick<CustomFieldDefinition, "kind" | "key">,
  format?: string | null,
) {
  const key =
    format === "FLAC" || format === "Ogg"
      ? field.key.toUpperCase()
      : format === "Itunes"
        ? field.key.replace(/^----:com\.apple\.iTunes:/, "")
        : field.key;
  const kind = ["FLAC", "Ogg", "Itunes"].includes(format ?? "")
    ? "text"
    : field.kind;
  return `custom:${kind}:${key}`;
}

export function customColumnValue(
  definition: CustomFieldDefinition,
  metadata: { key: string; value: string; ord: number }[],
  format?: string | null,
) {
  return metadata
    .filter((item) => item.key === fieldStorageKey(definition, format))
    .sort((a, b) => a.ord - b.ord)
    .map((item) => item.value)
    .join("; ");
}
