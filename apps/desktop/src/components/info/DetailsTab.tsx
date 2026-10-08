import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import toast from "react-hot-toast";
import type { FilesResponse } from "../../hooks/useFileWatcher";

import {
  saveMetadataWithCover,
  type StoredArtwork,
} from "../../utils/saveMetadata";
import type { DownloadedArtwork } from "../../utils/itunesClient";
import { useDetailsFields } from "../../hooks/useDetailsFields";
import { useMetadataFieldCatalog } from "../../hooks/useMetadataFieldCatalog";
import {
  sharedMetadataValues as getSharedValue,
  textValuesChange,
} from "../../utils/metadataFields";
import { useStore } from "../../hooks/useStore";
import ItunesLookup from "./ItunesLookup";
import { metadataFromItunes } from "../../utils/itunesMetadata";

const defaultMetadataFields = [
  { key: "title", label: "Title" },
  { key: "artist", label: "Artist" },
  { key: "album", label: "Album" },
  { key: "albumArtist", label: "Album Artist" },
  { key: "genre", label: "Genre" },
  { key: "year", label: "Year" },
  { key: "trackNumber", label: "Track Number" },
  { key: "discnumber", label: "Disc Number" },
  { key: "composer", label: "Composer" },
  { key: "comments", label: "Comments" },
] as const;

type MetadataKey = string;
type DetailField = {
  key: string;
  storageKey: string;
  label: string;
  multiValue: boolean;
};
type Draft = Record<MetadataKey, string[]>;
type TagChange =
  | { operation: "replace"; values: { type: "Text"; value: string }[] }
  | { operation: "delete" };
type UpdateMetadataResult = {
  updatedFileIds: number[];
  failures: { fileId: number; path: string; message: string }[];
};

const multiValueKeys = new Set<MetadataKey>([
  "artist",
  "albumArtist",
  "genre",
  "composer",
  "comments",
]);

function makeDraft(files: FilesResponse, metadataFields: DetailField[]): Draft {
  return Object.fromEntries(
    metadataFields.map(({ key, storageKey }) => [
      key,
      getSharedValue(files, storageKey).value,
    ]),
  ) as Draft;
}

export default function DetailsTab({
  files,
}: {
  files: FilesResponse | undefined;
}) {
  if (!files?.files.length) return null;

  return (
    <DetailsForm
      key={files.files.map((file) => file.id).join(",")}
      files={files}
    />
  );
}

function DetailsForm({ files }: { files: FilesResponse }) {
  const queryClient = useQueryClient();
  const { openSettings } = useStore();
  const selected = useDetailsFields();
  const catalog = useMetadataFieldCatalog();
  const metadataFields = useMemo<DetailField[]>(() => {
    const fields =
      catalog.data?.fields ??
      defaultMetadataFields.map((field) => ({
        ...field,
        key: field.key === "discnumber" ? "discNumber" : field.key,
        storageKey: field.key,
        multiValue: multiValueKeys.has(field.key),
        editable: true,
      }));
    return selected.fields.flatMap((key) => {
      const field = fields.find((field) => field.key === key && field.editable);
      return field ? [field] : [];
    });
  }, [catalog.data, selected.fields]);
  const initialDraft = useMemo(
    () => makeDraft(files, metadataFields),
    [files, metadataFields],
  );
  const mixedKeys = useMemo(
    () =>
      new Set(
        metadataFields
          .filter(({ storageKey }) => getSharedValue(files, storageKey).mixed)
          .map(({ key }) => key),
      ),
    [files, metadataFields],
  );
  const [importing, setImporting] = useState(false);
  const [lookupOpen, setLookupOpen] = useState(false);
  const [draft, setDraft] = useState<Draft>(initialDraft);
  const [dirtyKeys, setDirtyKeys] = useState<Set<MetadataKey>>(new Set());

  useEffect(() => {
    if (dirtyKeys.size) return;
    setDraft(initialDraft);
  }, [initialDraft, dirtyKeys.size]);

  const mutation = useMutation({
    mutationFn: ({
      changes,
      image,
    }: {
      changes: Record<MetadataKey, TagChange>;
      image?: DownloadedArtwork;
    }) =>
      saveMetadataWithCover(
        files.files.map((file) => file.id),
        changes,
        image,
        (fileIds) =>
          invoke<StoredArtwork[]>("get_artwork_details", { fileIds }),
        (input) => invoke<UpdateMetadataResult>("update_metadata", { input }),
      ),
    onSuccess: async (result, { changes, image }) => {
      if (result.updatedFileIds.length) {
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ["mediaFiles"] }),
          queryClient.invalidateQueries({ queryKey: ["customFields"] }),
          queryClient.invalidateQueries({ queryKey: ["metadataFieldCatalog"] }),
          ...(image
            ? [
                queryClient.invalidateQueries({ queryKey: ["artworkDetails"] }),
                queryClient.invalidateQueries({ queryKey: ["libraryArtwork"] }),
              ]
            : []),
        ]);
      }
      if (result.failures.length) {
        const first = result.failures[0];
        const file = first.path.split(/[\\/]/).pop() || `File ${first.fileId}`;
        toast.error(
          `${result.updatedFileIds.length} updated, ${result.failures.length} failed. ${file}: ${first.message}`,
          { duration: 7000 },
        );
        return;
      }
      const savedKeys = Object.keys(changes) as MetadataKey[];
      setDraft((current) => {
        const next = { ...current };
        for (const key of savedKeys) {
          const change = changes[key]!;
          next[key] =
            change.operation === "delete"
              ? []
              : change.values.map((item) => item.value);
        }
        return next;
      });
      setDirtyKeys(
        (current) =>
          new Set([...current].filter((key) => !savedKeys.includes(key))),
      );
      toast.success(
        `${result.updatedFileIds.length} file${result.updatedFileIds.length === 1 ? "" : "s"} updated`,
      );
    },
    onError: (error) =>
      toast.error(`Could not update metadata: ${String(error)}`),
  });

  const updateField = (key: MetadataKey, value: string[]) => {
    setDraft((current) => ({ ...current, [key]: value }));
    setDirtyKeys((current) => {
      const next = new Set(current);
      if (
        !mixedKeys.has(key) &&
        JSON.stringify(value.filter((item) => item !== "")) ===
          JSON.stringify(initialDraft[key] ?? [])
      )
        next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const reset = () => {
    setDraft(initialDraft);
    setDirtyKeys(new Set());
  };

  const save = () => {
    const changes: Record<MetadataKey, TagChange> = {};
    for (const key of dirtyKeys) {
      changes[key] = textValuesChange(draft[key] ?? []);
    }
    mutation.mutate({ changes });
  };

  return (
    <div className="flex min-h-full flex-col">
      <div className="px-6 pt-5">
        <button
          type="button"
          onClick={() => openSettings("metadata")}
          className="mb-3 rounded-lg border border-border px-3 py-2 text-sm"
        >
          Choose Details fields…
        </button>
        {lookupOpen ? (
          <ItunesLookup
            title={
              (draft.title ?? getSharedValue(files, "title").value)[0] ?? ""
            }
            artist={
              (draft.artist ?? getSharedValue(files, "artist").value)[0] ?? ""
            }
            disabled={mutation.isPending || importing}
            onClose={() => setLookupOpen(false)}
            onBusyChange={setImporting}
            onMetadata={async (match, image) => {
              const imported = metadataFromItunes(match);
              const changes: Record<MetadataKey, TagChange> = {};
              for (const [key, value] of Object.entries(imported)) {
                changes[key as MetadataKey] = {
                  operation: "replace",
                  values: [{ type: "Text", value }],
                };
              }
              const result = await mutation.mutateAsync({ changes, image });
              if (result.failures.length)
                throw new Error(result.failures[0].message);
            }}
          />
        ) : (
          <button
            type="button"
            disabled={mutation.isPending || importing}
            onClick={() => setLookupOpen(true)}
            className="h-9 rounded-lg border border-border px-3 text-sm"
          >
            Find metadata online
          </button>
        )}
      </div>
      <div className="grid grid-cols-1 gap-x-5 gap-y-4 p-6 md:grid-cols-2">
        {metadataFields.map(({ key, storageKey, label, multiValue }) => {
          const field = getSharedValue(files, storageKey);
          const values = draft[key] ?? initialDraft[key] ?? [];
          const inputs = values.length ? values : [""];
          return (
            <fieldset
              key={key}
              disabled={mutation.isPending || importing}
              className="flex min-w-0 flex-col gap-1.5"
            >
              <legend className="mb-1.5 text-xs font-medium text-muted-foreground">
                {label}
              </legend>
              {inputs.map((value, index) => (
                <div key={index} className="flex items-center gap-2">
                  <input
                    aria-label={
                      index === 0 ? label : `${label} value ${index + 1}`
                    }
                    value={value}
                    placeholder={field.mixed ? "Mixed" : "Not set"}
                    onChange={(event) => {
                      const next = [...inputs];
                      next[index] = event.target.value;
                      updateField(key, next);
                    }}
                    className="h-10 min-w-0 flex-1 rounded-lg border border-border bg-background px-3 text-sm text-foreground outline-none transition-colors placeholder:text-muted-foreground/70 focus:border-primary/60"
                  />
                  {(multiValue || inputs.length > 1) && (
                    <button
                      type="button"
                      aria-label={`Remove ${label} value ${index + 1}`}
                      onClick={() =>
                        updateField(
                          key,
                          inputs.filter((_, i) => i !== index),
                        )
                      }
                      className="rounded-lg border border-border px-2 py-2 text-xs"
                    >
                      Remove
                    </button>
                  )}
                </div>
              ))}
              {multiValue && (
                <button
                  type="button"
                  onClick={() =>
                    setDraft((current) => ({
                      ...current,
                      [key]: [...inputs, ""],
                    }))
                  }
                  className="self-start rounded-lg border border-border px-3 py-1.5 text-xs"
                >
                  Add value
                </button>
              )}
            </fieldset>
          );
        })}

        <div className="md:col-span-2 mt-1 rounded-xl border border-border bg-muted/30 p-4">
          <p className="text-xs font-medium text-muted-foreground">
            {files.files.length === 1 ? "File" : "Selected files"}
          </p>
          <p className="mt-1 truncate text-sm text-foreground">
            {files.files.length === 1
              ? files.files[0].path
              : `${files.files.length} tracks selected`}
          </p>
        </div>
      </div>

      <div className="sticky bottom-0 mt-auto flex items-center justify-end gap-2 border-t border-border bg-background/95 px-6 py-4 backdrop-blur-sm">
        <button
          type="button"
          onClick={reset}
          disabled={!dirtyKeys.size || mutation.isPending || importing}
          className="h-9 rounded-lg px-4 text-sm text-muted-foreground transition-colors hover:bg-muted disabled:cursor-not-allowed disabled:opacity-40"
        >
          Reset
        </button>
        <button
          type="button"
          onClick={save}
          disabled={!dirtyKeys.size || mutation.isPending || importing}
          className="h-9 rounded-lg bg-primary px-4 text-sm font-medium text-primary-foreground transition-opacity disabled:cursor-not-allowed disabled:opacity-40"
        >
          {mutation.isPending ? "Saving…" : "Save Changes"}
        </button>
      </div>
    </div>
  );
}
