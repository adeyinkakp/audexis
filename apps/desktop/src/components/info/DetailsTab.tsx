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
import ItunesLookup from "./ItunesLookup";
import { metadataFromItunes } from "../../utils/itunesMetadata";

const metadataFields = [
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

type MetadataKey = (typeof metadataFields)[number]["key"];
type Draft = Record<MetadataKey, string>;
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

function getSharedValue(files: FilesResponse, key: string) {
  const values = files.files.map((file) =>
    files.metadata
      .filter((item) => item.file_id === file.id && item.key === key)
      .sort((a, b) => a.ord - b.ord)
      .map((item) => item.value.trim())
      .filter(Boolean)
      .join("; "),
  );

  const first = values[0] ?? "";
  return {
    value: values.every((value) => value === first) ? first : "",
    mixed: values.some((value) => value !== first),
  };
}

function makeDraft(files: FilesResponse): Draft {
  return Object.fromEntries(
    metadataFields.map(({ key }) => [key, getSharedValue(files, key).value]),
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
  const initialDraft = useMemo(() => makeDraft(files), [files]);
  const mixedKeys = useMemo(
    () =>
      new Set(
        metadataFields
          .filter(({ key }) => getSharedValue(files, key).mixed)
          .map(({ key }) => key),
      ),
    [files],
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
      changes: Partial<Record<MetadataKey, TagChange>>;
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
              ? ""
              : change.values.map((item) => item.value).join("; ");
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

  const updateField = (key: MetadataKey, value: string) => {
    setDraft((current) => ({ ...current, [key]: value }));
    setDirtyKeys((current) => {
      const next = new Set(current);
      if (!mixedKeys.has(key) && value === initialDraft[key]) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const reset = () => {
    setDraft(initialDraft);
    setDirtyKeys(new Set());
  };

  const save = () => {
    const changes: Partial<Record<MetadataKey, TagChange>> = {};
    for (const key of dirtyKeys) {
      const value = draft[key].trim();
      if (!value) {
        changes[key] = { operation: "delete" };
        continue;
      }
      const values = (multiValueKeys.has(key) ? value.split(";") : [value])
        .map((part) => part.trim())
        .filter(Boolean)
        .map((part) => ({ type: "Text" as const, value: part }));
      changes[key] = { operation: "replace", values };
    }
    mutation.mutate({ changes });
  };

  return (
    <div className="flex min-h-full flex-col">
      <div className="px-6 pt-5">
        {lookupOpen ? (
          <ItunesLookup
            title={draft.title}
            artist={draft.artist}
            disabled={mutation.isPending || importing}
            onClose={() => setLookupOpen(false)}
            onBusyChange={setImporting}
            onMetadata={async (match, image) => {
              const imported = metadataFromItunes(match);
              const changes: Partial<Record<MetadataKey, TagChange>> = {};
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
        {metadataFields.map(({ key, label }) => {
          const field = getSharedValue(files, key);
          return (
            <label key={key} className="flex min-w-0 flex-col gap-1.5">
              <span className="text-xs font-medium text-muted-foreground">
                {label}
              </span>
              <input
                disabled={mutation.isPending || importing}
                aria-label={label}
                value={draft[key]}
                placeholder={field.mixed ? "Mixed" : "Not set"}
                onChange={(event) => updateField(key, event.target.value)}
                className="h-10 w-full rounded-lg border border-border bg-background px-3 text-sm text-foreground outline-none transition-colors placeholder:text-muted-foreground/70 focus:border-primary/60"
              />
            </label>
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
