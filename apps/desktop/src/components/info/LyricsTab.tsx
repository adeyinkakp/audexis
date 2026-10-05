import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import toast from "react-hot-toast";
import { parseLyrics } from "./synchronizedLyrics";
import LyricsLookup from "./LyricsLookup";
import SynchronizedLyricsEditor from "./SynchronizedLyricsEditor";
import type { FilesResponse } from "../../hooks/useFileWatcher";

type LyricsInfo = {
  fileId: number;
  plain: string[];
  synced: string[];
  supported: boolean;
};
type SaveResult = {
  updatedFileIds: number[];
  failures: { path: string; message: string }[];
};

export default function LyricsTab({
  files,
  synchronized = false,
  active = true,
}: {
  files: FilesResponse;
  synchronized?: boolean;
  active?: boolean;
}) {
  const fileIds = files.files.map((file) => file.id);
  const query = useQuery({
    queryKey: ["lyrics", fileIds],
    queryFn: () => invoke<LyricsInfo[]>("get_lyrics", { fileIds }),
    enabled: fileIds.length > 0,
  });
  if (query.isPending)
    return (
      <p className="p-6 text-sm text-muted-foreground">
        Loading embedded lyrics…
      </p>
    );
  if (query.isError)
    return (
      <div className="p-6 text-sm">
        <p role="alert">Could not read lyrics: {String(query.error)}</p>
        <button
          className="mt-3 rounded-lg border border-border px-3 py-2"
          onClick={() => void query.refetch()}
        >
          Retry
        </button>
      </div>
    );
  if (!query.data.length)
    return <p className="p-6 text-sm">No tracks selected.</p>;
  return (
    <LyricsEditor
      files={files}
      lyrics={query.data}
      synchronized={synchronized}
      active={active}
    />
  );
}

function LyricsEditor({
  files,
  lyrics,
  synchronized,
  active,
}: {
  files: FilesResponse;
  lyrics: LyricsInfo[];
  synchronized: boolean;
  active: boolean;
}) {
  const queryClient = useQueryClient();
  const [lookupOpen, setLookupOpen] = useState(false);
  const firstFile = files.files[0];
  const title =
    files.metadata.find(
      (item) => item.file_id === firstFile.id && item.key === "title",
    )?.value || firstFile.file_name;
  const artist =
    files.metadata.find(
      (item) => item.file_id === firstFile.id && item.key === "artist",
    )?.value || "";
  const kind = synchronized ? "synced" : "plain";
  const key = synchronized ? "synchronizedLyrics" : "unsyncedLyrics";
  const initial = lyrics[0]?.[kind][0] ?? "";
  const [draft, setDraft] = useState<string | null>(null);
  const value = draft ?? initial;
  const mixed = lyrics.some(
    (item) => JSON.stringify(item[kind]) !== JSON.stringify(lyrics[0][kind]),
  );
  const multiple = lyrics.some((item) => item[kind].length > 1);
  const supported = lyrics.every((item) => item.supported);
  const invalid = synchronized && parseLyrics(value).error !== null;
  const dirty = draft !== null && (draft !== initial || mixed || multiple);
  const mutation = useMutation({
    mutationFn: () =>
      invoke<SaveResult>("update_metadata", {
        input: {
          fileIds: files.files.map((file) => file.id),
          changes: {
            [key]: value.trim()
              ? { operation: "replace", values: [{ type: "Text", value }] }
              : { operation: "delete" },
          },
        },
      }),
    onSuccess: async (result) => {
      if (result.updatedFileIds.length)
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ["lyrics"] }),
          queryClient.invalidateQueries({ queryKey: ["mediaFiles"] }),
        ]);
      if (result.failures.length) {
        toast.error(
          `${result.updatedFileIds.length} saved, ${result.failures.length} failed. ${result.failures[0].message}`,
          { duration: 7000 },
        );
        return;
      }
      setDraft(null);
      toast.success(
        `Lyrics saved to ${result.updatedFileIds.length} file${result.updatedFileIds.length === 1 ? "" : "s"}`,
      );
    },
    onError: (error) =>
      toast.error(`Could not save lyrics: ${String(error)}`, {
        duration: 7000,
      }),
  });
  return (
    <div className="flex min-h-full flex-col">
      <div className="flex flex-1 flex-col gap-4 p-6">
        {!synchronized && (
          <div>
            <h2 className="text-sm font-semibold">
              {synchronized ? "Synchronized lyrics" : "Lyrics"}
            </h2>
            <p
              id={`${key}-help`}
              className="mt-1 text-xs text-muted-foreground"
            >
              {synchronized
                ? "Play the track, edit each line, and adjust its timing. Preview follows along as you listen. Use Raw LRC to view or edit the timestamped text."
                : "Edit the lyrics embedded in your audio file. Line breaks and punctuation are preserved."}{" "}
              Clear the text and save to remove it.
            </p>
          </div>
        )}
        {files.files.length > 1 && (
          <p className="rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-xs">
            {mixed
              ? "These tracks have different lyrics. Showing the first track. "
              : ""}
            Saving applies this text to all {files.files.length} selected
            tracks.
          </p>
        )}
        {multiple && (
          <p className="text-xs text-amber-600">
            Multiple embedded entries exist. Showing the first; saving replaces
            all entries for this lyrics type.
          </p>
        )}
        {!supported && (
          <p role="alert" className="text-sm text-destructive">
            One or more selected formats do not support lyrics editing.
          </p>
        )}
        <div className="flex justify-end">
          <button
            type="button"
            onClick={() => setLookupOpen((open) => !open)}
            aria-expanded={lookupOpen}
            disabled={!supported || mutation.isPending}
            className="rounded-lg border border-border px-3 py-2 text-xs font-medium hover:bg-muted disabled:opacity-40"
          >
            Find lyrics online
          </button>
        </div>
        {lookupOpen && active && (
          <LyricsLookup
            key={`${firstFile.id}-${kind}`}
            title={title}
            artist={artist}
            synchronized={synchronized}
            hasDraft={!!value.trim()}
            disabled={!supported || mutation.isPending}
            onClose={() => setLookupOpen(false)}
            onApply={(text) => {
              setDraft(text);
              setLookupOpen(false);
              toast.success(
                "Lyrics added to the editor. Click Save Lyrics to keep them.",
              );
            }}
          />
        )}
        {synchronized ? (
          <SynchronizedLyricsEditor
            value={value}
            onChange={setDraft}
            fileId={files.files[0].id}
            title={title}
            artist={artist}
            disabled={!supported || mutation.isPending}
            active={active}
          />
        ) : (
          <textarea
            aria-label={synchronized ? "Synchronized lyrics (LRC)" : "Lyrics"}
            aria-describedby={`${key}-help`}
            spellCheck={!synchronized}
            value={value}
            disabled={!supported || mutation.isPending}
            onChange={(event) => setDraft(event.target.value)}
            placeholder={
              synchronized
                ? "[00:12.500]First line\n[00:17.250]Next line"
                : "Enter lyrics…"
            }
            className={`min-h-72 flex-1 resize-y rounded-xl border border-border bg-background p-4 text-sm leading-7 outline-none focus:border-primary/60 disabled:opacity-50 ${synchronized ? "font-mono" : ""}`}
          />
        )}
        <p className="truncate text-xs text-muted-foreground">
          {files.files.length === 1
            ? files.files[0].path
            : `${files.files.length} tracks selected`}
        </p>
      </div>
      <div className="sticky bottom-0 mt-auto flex justify-end gap-2 border-t border-border bg-background/95 px-6 py-4">
        <button
          type="button"
          onClick={() => setDraft("")}
          disabled={!value || !supported || mutation.isPending}
          className="mr-auto h-9 rounded-lg px-4 text-sm text-muted-foreground hover:bg-muted disabled:opacity-40"
        >
          Clear
        </button>
        <button
          type="button"
          onClick={() => setDraft(null)}
          disabled={draft === null || mutation.isPending}
          className="h-9 rounded-lg px-4 text-sm text-muted-foreground hover:bg-muted disabled:opacity-40"
        >
          Reset
        </button>
        <button
          type="button"
          onClick={() => mutation.mutate()}
          disabled={!dirty || !supported || mutation.isPending || invalid}
          className="h-9 rounded-lg bg-primary px-4 text-sm font-medium text-primary-foreground disabled:opacity-40"
        >
          {mutation.isPending ? "Saving…" : "Save Lyrics"}
        </button>
      </div>
    </div>
  );
}
