import { LibrarySearchInput } from "../../components/library/LibrarySearchInput";
import { Artwork } from "../../components/library/CollectionGrid";
import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { Heart, Plus } from "lucide-react";
import { useState } from "react";
import { useCreatePlaylist, usePlaylists } from "../../hooks/usePlaylists";
import { PlaylistNameModal } from "../../modals/PlaylistNameModal";
export const Route = createFileRoute("/_noneditor/playlists/")({
  validateSearch: (search: Record<string, unknown>): { q?: string } => ({
    q: typeof search.q === "string" ? search.q : undefined,
  }),
  component: PlaylistsIndexPage,
});
function PlaylistsIndexPage() {
  const { q = "" } = Route.useSearch();
  const result = usePlaylists();
  const terms = q.toLocaleLowerCase().split(/\s+/).filter(Boolean);
  const playlists =
    result.data?.filter((playlist) =>
      terms.every((term) => playlist.name.toLocaleLowerCase().includes(term)),
    ) ?? [];
  const showFavorites = terms.every((term) => "favorites".includes(term));
  const createPlaylist = useCreatePlaylist();
  const navigate = useNavigate();
  const [creating, setCreating] = useState(false);
  return (
    <main className="h-[calc(100dvh-5.5rem)] overflow-auto px-8 pb-10 pt-7">
      <header className="mb-8 flex flex-wrap items-end justify-between gap-5 border-b border-border/60 pb-6">
        <div>
          <p className="mb-2 text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
            Your library
          </p>
          <h1 className="text-4xl font-bold tracking-tight">Playlists</h1>
        </div>
        <LibrarySearchInput
          value={q}
          label="Search playlists"
          onChange={(value) =>
            void navigate({
              to: "/playlists",
              search: { q: value || undefined },
              replace: true,
            })
          }
        />
      </header>
      <div className="grid grid-cols-[repeat(auto-fill,minmax(170px,1fr))] gap-x-6 gap-y-8">
        <button
          type="button"
          onClick={() => setCreating(true)}
          className="group min-w-0 text-left focus-visible:outline-primary"
        >
          <div className="flex aspect-square items-center justify-center rounded-xl border border-border bg-muted/30 transition-colors group-hover:border-primary/50 group-hover:bg-primary/10">
            <Plus
              size={64}
              strokeWidth={1.3}
              className="text-primary transition-transform group-hover:scale-110"
            />
          </div>
          <p className="mt-3 font-medium">Create Playlist</p>
          <p className="mt-1 text-sm text-muted-foreground">
            Make something yours
          </p>
        </button>
        {showFavorites && (
          <Link
            to="/favorites"
            className="group min-w-0 rounded-xl focus-visible:outline-primary"
          >
            <div className="flex aspect-square items-center justify-center rounded-xl bg-gradient-to-br from-rose-400 via-pink-500 to-violet-600 text-white shadow-sm transition-shadow group-hover:shadow-lg">
              <Heart size={64} fill="currentColor" strokeWidth={1} />
            </div>
            <p className="mt-3 font-medium">Favorites</p>
            <p className="mt-1 text-sm text-muted-foreground">
              The songs you love
            </p>
          </Link>
        )}
        {playlists.map((playlist) => (
          <Link
            key={playlist.id}
            to="/playlists/$playlistId"
            params={{ playlistId: String(playlist.id) }}
            className="group min-w-0 rounded-xl focus-visible:outline-primary"
          >
            <div className="rounded-xl shadow-sm transition-shadow group-hover:shadow-lg">
              <Artwork id={playlist.artwork_file_id ?? null} />
            </div>
            <p className="mt-3 truncate font-medium" title={playlist.name}>
              {playlist.name}
            </p>
            <p className="mt-1 text-sm text-muted-foreground">
              {playlist.track_count}{" "}
              {playlist.track_count === 1 ? "song" : "songs"}
            </p>
          </Link>
        ))}
      </div>
      {result.isPending && (
        <p className="py-8 text-sm text-muted-foreground">Loading playlists…</p>
      )}
      {result.isError && (
        <button
          onClick={() => void result.refetch()}
          className="mt-8 text-sm text-destructive"
        >
          Could not load playlists. Try again
        </button>
      )}
      {!result.isPending && !result.isError && !playlists.length && (
        <p className="py-8 text-sm text-muted-foreground">
          {q.trim() && "No matching custom playlists. Try another name."}
        </p>
      )}
      {creating && (
        <PlaylistNameModal
          onClose={() => setCreating(false)}
          onSave={async (name) => {
            const id = await createPlaylist.mutateAsync(name);
            void navigate({
              to: "/playlists/$playlistId",
              params: { playlistId: String(id) },
            });
          }}
        />
      )}
    </main>
  );
}
