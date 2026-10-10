import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import toast from "react-hot-toast";
import type { FilesResponse } from "../../hooks/useFileWatcher";
import { useStore } from "../../hooks/useStore";
import { useCustomFieldDefinitions } from "../../hooks/useCustomFieldDefinitions";
import {
  customFieldChanges,
  customFieldRows,
  fieldStorageKey,
  validateCustomFields,
  type CustomField,
  type CustomFields,
} from "../../utils/customFields";

const inputClass =
  "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm";
const buttonClass =
  "rounded-lg border border-border px-3 py-2 text-sm disabled:opacity-40";

export default function CustomFieldsTab({ files }: { files: FilesResponse }) {
  const [selectedId, setSelectedId] = useState(files.files[0]?.id);
  const [locked, setLocked] = useState(false);
  const { openCustomFieldSettings } = useStore();
  if (!selectedId) return null;
  return (
    <div className="space-y-4 p-6">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-muted-foreground">
          Add, edit, or remove metadata fields for this song.
        </p>
        <button
          className={`${buttonClass} shrink-0`}
          onClick={() => openCustomFieldSettings()}
        >
          Manage Saved Fields
        </button>
      </div>
      {files.files.length > 1 && (
        <label className="block text-sm">
          Edit fields for
          <select
            aria-label="Track to edit"
            className={`${inputClass} mt-2`}
            value={selectedId}
            disabled={locked}
            onChange={(event) => setSelectedId(Number(event.target.value))}
          >
            {files.files.map((file) => (
              <option key={file.id} value={file.id}>
                {file.file_name}
              </option>
            ))}
          </select>
        </label>
      )}
      <h2 id="custom-metadata-fields" className="text-base font-semibold">
        Custom Text / URL Fields
      </h2>
      <CustomFieldsForm
        key={selectedId}
        fileId={selectedId}
        onLock={setLocked}
      />
    </div>
  );
}

function CustomFieldsForm({
  fileId,
  onLock,
}: {
  fileId: number;
  onLock: (locked: boolean) => void;
}) {
  const client = useQueryClient();
  const { openCustomFieldSettings } = useStore();
  const definitions = useCustomFieldDefinitions();
  const fields = useQuery({
    queryKey: ["customFields", fileId],
    queryFn: () => invoke<CustomFields>("get_custom_fields", { fileId }),
    gcTime: 0,
  });
  const [draft, setDraft] = useState<CustomField[] | null>(null);
  const rows =
    draft ??
    (fields.data ? customFieldRows(fields.data, definitions.definitions) : []);
  const error = validateCustomFields(rows, fields.data?.format);
  const edit = (next: CustomField[]) => {
    setDraft(next);
    onLock(true);
  };
  const reset = () => {
    setDraft(null);
    onLock(false);
  };
  const save = useMutation({
    mutationFn: () =>
      invoke<{
        updatedFileIds: number[];
        failures: { message: string }[];
      }>("update_metadata", {
        input: {
          fileIds: [fileId],
          changes: customFieldChanges(rows, fields.data?.supportsUrls),
        },
      }),
    onSuccess: async (result) => {
      if (result.failures.length) {
        toast.error(result.failures[0].message);
        return;
      }
      await Promise.all([
        client.invalidateQueries({ queryKey: ["customFields", fileId] }),
        client.invalidateQueries({ queryKey: ["mediaFiles"] }),
        client.invalidateQueries({ queryKey: ["fileWatcherMap"] }),
        client.invalidateQueries({ queryKey: ["searchMedia"] }),
        client.invalidateQueries({ queryKey: ["metadataFieldCatalog"] }),
      ]);
      reset();
      toast.success("Custom values saved to the audio file");
    },
    onError: (error) =>
      toast.error(`Could not save custom fields: ${String(error)}`),
  });

  if (fields.isPending)
    return <p className="text-sm">Loading custom fields…</p>;
  if (fields.isError)
    return (
      <div role="alert" className="text-sm">
        <p>Could not load custom fields: {String(fields.error)}</p>
        <button
          className={`${buttonClass} mt-3`}
          onClick={() => void fields.refetch()}
        >
          Retry
        </button>
      </div>
    );
  if (!fields.data.supported)
    return (
      <p className="text-sm text-muted-foreground">
        This file’s metadata format ({fields.data.format}) cannot store custom
        fields with this editor.
      </p>
    );
  const identity = (kind: CustomField["kind"], key: string) =>
    fieldStorageKey({ kind, key }, fields.data.format);
  return (
    <div className="space-y-4">
      {definitions.isError ? (
        <p role="alert" className="text-sm">
          Could not load saved field definitions.{" "}
          <button onClick={() => void definitions.refetch()}>Retry</button>
        </p>
      ) : (
        <label className="block text-sm">
          Add a saved field
          <select
            className={`${inputClass} mt-1`}
            value=""
            disabled={save.isPending || definitions.isPending}
            onChange={(event) => {
              const field = definitions.definitions.find(
                (item) => item.id === event.target.value,
              );
              if (field)
                edit([
                  ...rows,
                  {
                    id: crypto.randomUUID(),
                    name: field.key,
                    kind: field.kind,
                    value: "",
                  },
                ]);
            }}
          >
            <option value="">Choose a field…</option>
            {definitions.definitions.map((field) => (
              <option
                key={field.id}
                value={field.id}
                disabled={rows.some(
                  (row) =>
                    identity(row.kind, row.name) ===
                    identity(field.kind, field.key),
                )}
              >
                {field.title} ({field.key})
              </option>
            ))}
          </select>
        </label>
      )}
      {!rows.length && (
        <p className="text-sm text-muted-foreground">
          No custom values on this song yet.
        </p>
      )}
      {rows.map((row, index) => {
        const definition = definitions.definitions.find(
          (field) =>
            identity(field.kind, field.key) === identity(row.kind, row.name),
        );
        const update = (change: Partial<CustomField>) =>
          edit(
            rows.map((item) =>
              item.id === row.id ? { ...item, ...change } : item,
            ),
          );
        return (
          <fieldset
            key={row.id}
            disabled={save.isPending}
            className="space-y-3 rounded-xl border border-border p-4"
          >
            <legend className="px-1 text-sm font-medium">
              {definition?.title || `Field ${index + 1}`}
            </legend>
            <div className="flex items-end gap-3">
              <label className="w-28 shrink-0 text-xs">
                Type
                <select
                  className={`${inputClass} mt-1`}
                  value={row.kind}
                  onChange={(event) =>
                    update({ kind: event.target.value as CustomField["kind"] })
                  }
                >
                  <option value="text">Text</option>
                  <option value="url">URL</option>
                </select>
              </label>
              <label className="min-w-0 flex-1 text-xs">
                Metadata key
                <input
                  className={`${inputClass} mt-1`}
                  value={row.name}
                  placeholder="e.g. MOOD"
                  onChange={(event) => update({ name: event.target.value })}
                />
              </label>
              <button
                type="button"
                className={buttonClass}
                aria-label={`Remove field ${index + 1}`}
                onClick={() => edit(rows.filter((item) => item.id !== row.id))}
              >
                Remove
              </button>
            </div>
            <label className="block text-xs">
              Value for this song
              <textarea
                rows={2}
                className={`${inputClass} mt-1 resize-y`}
                value={row.value}
                onChange={(event) => update({ value: event.target.value })}
              />
            </label>
            {!definition && (
              <button
                className={buttonClass}
                disabled={!row.name.trim()}
                onClick={() =>
                  openCustomFieldSettings({ key: row.name, kind: row.kind })
                }
              >
                Save Field Definition…
              </button>
            )}
          </fieldset>
        );
      })}
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
      <div className="flex items-center gap-2">
        <button
          type="button"
          className={buttonClass}
          disabled={save.isPending}
          onClick={() =>
            edit([
              ...rows,
              { id: crypto.randomUUID(), kind: "text", name: "", value: "" },
            ])
          }
        >
          Add Field
        </button>
        <div className="flex-1" />
        <button
          type="button"
          className={buttonClass}
          disabled={!draft || save.isPending}
          onClick={reset}
        >
          Reset
        </button>
        <button
          type="button"
          className="rounded-lg bg-primary px-4 py-2 text-sm text-primary-foreground disabled:opacity-40"
          disabled={!draft || !!error || save.isPending}
          onClick={() => save.mutate()}
        >
          {save.isPending ? "Saving…" : "Save Song Values"}
        </button>
      </div>
    </div>
  );
}
