import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Image as ImageIcon } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import toast from "react-hot-toast";
import type { FilesResponse } from "../../hooks/useFileWatcher";
import { cn } from "../../utils";

type ArtworkInfo = {
  id: number;
  file_id: number;
  data_url: string;
  data_base64: string;
  mime_type: string;
  picture_type: number;
  description: string;
};

type ArtworkItem = {
  id: string;
  mime: string;
  data_base64: string;
  picture_type: number;
  description: string;
};

type ImportedArtwork = Omit<ArtworkItem, "id">;

const pictureTypes = [
  { id: 3, name: "Front cover" },
  { id: 4, name: "Back cover" },
  { id: 0, name: "Other" },
  { id: 5, name: "Leaflet" },
  { id: 6, name: "Media" },
  { id: 7, name: "Lead artist" },
  { id: 8, name: "Artist" },
  { id: 9, name: "Conductor" },
  { id: 10, name: "Band" },
  { id: 11, name: "Composer" },
  { id: 12, name: "Lyricist" },
  { id: 13, name: "Recording location" },
  { id: 18, name: "Illustration" },
  { id: 19, name: "Band logo" },
  { id: 20, name: "Publisher logo" },
] as const;

function imageKey(item: ArtworkInfo) {
  return [
    item.mime_type,
    item.data_base64,
    item.picture_type,
    item.description,
  ].join("|");
}

function editableItems(artwork: ArtworkInfo[], firstFileId: number) {
  return artwork
    .filter((item) => item.file_id === firstFileId)
    .map((item) => ({
      id: `stored-${item.id}`,
      mime: item.mime_type,
      data_base64: item.data_base64,
      picture_type: item.picture_type,
      description: item.description,
    }));
}

function hasMixedArtwork(artwork: ArtworkInfo[], fileIds: number[]) {
  if (fileIds.length < 2) return false;
  const first = artwork
    .filter((item) => item.file_id === fileIds[0])
    .map(imageKey);
  return fileIds.slice(1).some((fileId) => {
    const current = artwork
      .filter((item) => item.file_id === fileId)
      .map(imageKey);
    return (
      current.length !== first.length ||
      current.some((value, index) => value !== first[index])
    );
  });
}

export default function ArtworkTab({ files }: { files: FilesResponse | undefined }) {
  const queryClient = useQueryClient();
  const fileIds = useMemo(
    () => files?.files.map((file) => file.id) ?? [],
    [files],
  );
  const query = useQuery({
    queryKey: ["artworkDetails", fileIds],
    queryFn: () => invoke<ArtworkInfo[]>("get_artwork_details", { fileIds }),
    enabled: fileIds.length > 0,
    staleTime: Infinity,
  });
  const initialItems = useMemo(
    () => editableItems(query.data ?? [], fileIds[0] ?? 0),
    [query.data, fileIds],
  );
  const mixed = useMemo(
    () => hasMixedArtwork(query.data ?? [], fileIds),
    [query.data, fileIds],
  );
  const [items, setItems] = useState<ArtworkItem[]>(initialItems);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [dirty, setDirty] = useState(false);
  const selected = items[selectedIndex] ?? null;

  useEffect(() => {
    setItems(initialItems);
    setSelectedIndex(0);
    setDirty(false);
  }, [initialItems]);

  const mutation = useMutation({
    mutationFn: () =>
      invoke("update_metadata", {
        input: {
          fileIds,
          changes: {
            attachedPicture: items.length
              ? {
                  operation: "replace",
                  values: items.map((item) => ({
                    type: "Picture",
                    value: {
                      mime: item.mime,
                      data_base64: item.data_base64,
                      picture_type: item.picture_type,
                      description: item.description.trim() || undefined,
                    },
                  })),
                }
              : { operation: "delete" },
          },
        },
      }),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["artworkDetails"] }),
        queryClient.invalidateQueries({ queryKey: ["libraryArtwork"] }),
      ]);
      setDirty(false);
      toast.success("Artwork updated");
    },
    onError: (error) => toast.error(`Could not update artwork: ${String(error)}`),
  });

  const apply = (next: ArtworkItem[], nextIndex = selectedIndex) => {
    setItems(next);
    setSelectedIndex(Math.max(0, Math.min(nextIndex, next.length - 1)));
    setDirty(true);
  };

  const addImage = async () => {
    try {
      const image = await invoke<ImportedArtwork | null>("import_artwork");
      if (!image) return;
      const next = [...items, { ...image, id: `new-${Date.now()}` }];
      apply(next, next.length - 1);
    } catch (error) {
      toast.error(`Could not import artwork: ${String(error)}`);
    }
  };

  const updateSelected = (change: Partial<ArtworkItem>) => {
    if (!selected) return;
    apply(
      items.map((item, index) =>
        index === selectedIndex ? { ...item, ...change } : item,
      ),
    );
  };

  const move = (offset: number) => {
    const destination = selectedIndex + offset;
    if (!selected || destination < 0 || destination >= items.length) return;
    const next = [...items];
    const [item] = next.splice(selectedIndex, 1);
    next.splice(destination, 0, item);
    apply(next, destination);
  };

  if (query.isLoading) {
    return (
      <div className="flex h-80 items-center justify-center text-sm text-muted-foreground">
        Loading artwork…
      </div>
    );
  }

  return (
    <div className="flex min-h-full flex-col">
      {mixed && (
        <div className="mx-6 mt-5 rounded-xl border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-xs text-foreground/80">
          The selected tracks have different artwork. This shows the first
          track’s images; saving will apply this artwork to all selected tracks.
        </div>
      )}

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-5 p-6 md:grid-cols-[minmax(0,1fr)_17rem]">
        <section className="flex min-w-0 flex-col gap-3">
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="flex min-w-0 flex-col gap-1.5">
              <span className="text-xs font-medium text-muted-foreground">Current Type</span>
              <select
                disabled={!selected}
                value={selected?.picture_type ?? 3}
                onChange={(event) =>
                  updateSelected({ picture_type: Number(event.target.value) })
                }
                className="h-10 rounded-lg border border-border bg-background px-3 text-sm outline-none disabled:opacity-50"
              >
                {pictureTypes.map((type) => (
                  <option key={type.id} value={type.id}>{type.name}</option>
                ))}
              </select>
            </label>
            <label className="flex min-w-0 flex-col gap-1.5">
              <span className="text-xs font-medium text-muted-foreground">Description</span>
              <input
                disabled={!selected}
                value={selected?.description ?? ""}
                onChange={(event) => updateSelected({ description: event.target.value })}
                placeholder="Optional description"
                className="h-10 rounded-lg border border-border bg-background px-3 text-sm outline-none disabled:opacity-50"
              />
            </label>
          </div>

          <div className="flex min-h-72 flex-1 items-center justify-center overflow-hidden rounded-2xl border border-border bg-muted/30 p-5">
            {selected ? (
              <img
                src={`data:${selected.mime};base64,${selected.data_base64}`}
                alt={selected.description || "Selected artwork"}
                className="max-h-[42vh] max-w-full rounded-lg object-contain shadow-lg"
              />
            ) : (
              <div className="flex flex-col items-center gap-3 text-muted-foreground">
                <ImageIcon size={42} strokeWidth={1.25} />
                <p className="text-sm">No embedded artwork</p>
              </div>
            )}
          </div>

          <div className="flex flex-wrap gap-2">
            <button type="button" onClick={() => move(-1)} disabled={selectedIndex === 0 || !selected} className="h-9 rounded-lg border border-border px-3 text-sm disabled:opacity-40">Up</button>
            <button type="button" onClick={() => move(1)} disabled={!selected || selectedIndex === items.length - 1} className="h-9 rounded-lg border border-border px-3 text-sm disabled:opacity-40">Down</button>
            <button type="button" onClick={() => move(-selectedIndex)} disabled={!selected || selectedIndex === 0} className="h-9 rounded-lg px-3 text-sm text-muted-foreground disabled:opacity-40">Make Primary</button>
          </div>
        </section>

        <aside className="flex min-h-0 flex-col rounded-2xl border border-border bg-muted/20 p-3">
          <div className="flex items-center justify-between px-1 pb-3">
            <h3 className="text-sm font-semibold text-foreground">Images</h3>
            <span className="rounded-full bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">{items.length}</span>
          </div>

          <div className="flex max-h-[43vh] flex-col gap-2 overflow-y-auto">
            {items.map((item, index) => (
              <button
                key={item.id}
                type="button"
                onClick={() => setSelectedIndex(index)}
                className={cn(
                  "flex w-full items-center gap-3 rounded-xl border border-transparent p-2 text-left transition-colors hover:bg-active",
                  selectedIndex === index && "border-primary/30 bg-primary/8",
                )}
              >
                <img src={`data:${item.mime};base64,${item.data_base64}`} alt="" className="h-14 w-14 shrink-0 rounded-lg bg-muted object-cover" />
                <span className="min-w-0">
                  <span className="block truncate text-sm font-medium text-foreground">{index === 0 ? "Primary" : `Image ${index + 1}`}</span>
                  <span className="block truncate text-xs text-muted-foreground">{pictureTypes.find((type) => type.id === item.picture_type)?.name ?? "Other"}</span>
                </span>
              </button>
            ))}
          </div>

          <div className="mt-auto grid grid-cols-2 gap-2 pt-3">
            <button type="button" onClick={() => void addImage()} className="h-9 rounded-lg bg-primary px-3 text-sm font-medium text-primary-foreground">Add image</button>
            <button
              type="button"
              onClick={() => apply(items.filter((_, index) => index !== selectedIndex), selectedIndex - 1)}
              disabled={!selected}
              className="h-9 rounded-lg border border-border px-3 text-sm disabled:opacity-40"
            >
              Remove
            </button>
          </div>
        </aside>
      </div>

      <div className="sticky bottom-0 mt-auto flex items-center justify-end gap-2 border-t border-border bg-background/95 px-6 py-4 backdrop-blur-sm">
        <button
          type="button"
          onClick={() => { setItems(initialItems); setSelectedIndex(0); setDirty(false); }}
          disabled={!dirty || mutation.isPending}
          className="h-9 rounded-lg px-4 text-sm text-muted-foreground hover:bg-muted disabled:opacity-40"
        >
          Reset
        </button>
        <button type="button" onClick={() => mutation.mutate()} disabled={!dirty || mutation.isPending} className="h-9 rounded-lg bg-primary px-4 text-sm font-medium text-primary-foreground disabled:opacity-40">
          {mutation.isPending ? "Saving…" : "Save Artwork"}
        </button>
      </div>
    </div>
  );
}
