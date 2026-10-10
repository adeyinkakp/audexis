import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { FolderSearch, Trash2 } from "lucide-react";
import toast from "react-hot-toast";

type MissingFile = {
  id: number;
  path: string;
  file_name: string;
  missing_since: number;
  title: string;
  artist: string;
};

export const Route = createFileRoute("/_noneditor/missing")({
  component: MissingFilesPage,
});

function MissingFilesPage() {
  const client = useQueryClient();
  const query = useQuery({
    queryKey: ["missingFiles"],
    queryFn: () => invoke<MissingFile[]>("get_missing_files"),
  });

  const relink = useMutation({
    mutationFn: (fileId: number) =>
      invoke<string | null>("relink_missing_file", { fileId }),
    onSuccess: (path) => {
      if (!path) return;
      void client.invalidateQueries({ queryKey: ["missingFiles"] });
      toast.success("File record relinked");
    },
    onError: (error) => toast.error(`Could not relink file: ${String(error)}`),
  });
  const remove = useMutation({
    mutationFn: (fileId: number) => invoke("delete_missing_file", { fileId }),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["missingFiles"] });
      toast.success("Missing file record deleted");
    },
    onError: (error) =>
      toast.error(`Could not delete record: ${String(error)}`),
  });

  const files = query.data ?? [];
  return (
    <main className="min-h-[calc(100dvh-3.5rem)] p-6 pb-28">
      <header className="mb-6">
        <div className="flex items-center gap-3">
          <h1 className="text-2xl font-semibold">Missing Files</h1>
          <span className="rounded-full bg-muted px-2.5 py-1 text-xs text-muted-foreground">
            {files.length}
          </span>
        </div>
        <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
          These tracks are no longer at their saved locations. Relink a track to
          preserve its favorites, playlists, and listening history, or delete
          its library record permanently.
        </p>
      </header>

      {query.isPending ? (
        <p className="text-sm text-muted-foreground">Loading missing files…</p>
      ) : files.length === 0 ? (
        <div className="rounded-2xl border border-border bg-muted/20 p-10 text-center">
          <p className="font-medium">No missing files</p>
          <p className="mt-1 text-sm text-muted-foreground">
            Every indexed track is currently available.
          </p>
        </div>
      ) : (
        <div className="space-y-3">
          {files.map((file) => (
            <article
              key={file.id}
              className="flex flex-col gap-4 rounded-2xl border border-border bg-background/70 p-4 md:flex-row md:items-center"
            >
              <div className="min-w-0 flex-1">
                <h2 className="truncate font-medium">
                  {file.title || file.file_name}
                </h2>
                {file.artist && (
                  <p className="truncate text-sm text-muted-foreground">
                    {file.artist}
                  </p>
                )}
                <p
                  className="mt-1 truncate text-xs text-muted-foreground/70"
                  title={file.path}
                >
                  {file.path}
                </p>
              </div>
              <div className="flex shrink-0 gap-2">
                <button
                  type="button"
                  onClick={() => relink.mutate(file.id)}
                  disabled={relink.isPending || remove.isPending}
                  className="flex h-9 items-center gap-2 rounded-lg bg-primary px-3 text-sm font-medium text-primary-foreground disabled:opacity-50"
                >
                  <FolderSearch size={15} />
                  Locate in Finder
                </button>
                <button
                  type="button"
                  onClick={() => {
                    remove.mutate(file.id);
                  }}
                  disabled={relink.isPending || remove.isPending}
                  className="flex h-9 items-center gap-2 rounded-lg border border-destructive/40 hover:bg-destructive hover:text-destructive-foreground  px-3 text-sm text-destructive disabled:opacity-50"
                >
                  <Trash2 size={15} />
                  Delete Record
                </button>
              </div>
            </article>
          ))}
        </div>
      )}
    </main>
  );
}
