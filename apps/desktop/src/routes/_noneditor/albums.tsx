import { LibrarySearchInput } from "../../components/library/LibrarySearchInput";
import { AlbumHeader } from "../../components/library/AlbumHeader";
import { createFileRoute, Link } from "@tanstack/react-router";
import { CollectionGrid } from "../../components/library/CollectionGrid";
import { CollectionSongs } from "../../components/library/CollectionSongs";
import { emptySearchFilters } from "../../hooks/useSearchMedia";
export const Route = createFileRoute("/_noneditor/albums")({
  validateSearch: (
    search: Record<string, unknown>,
  ): { album?: string; artist?: string; q?: string } => ({
    album: typeof search.album === "string" ? search.album : undefined,
    q: typeof search.q === "string" ? search.q : undefined,
    artist: typeof search.artist === "string" ? search.artist : undefined,
  }),
  component: AlbumsPage,
});
function AlbumsPage() {
  const { album, artist, q = "" } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <main className="h-[calc(100dvh-5.5rem)] overflow-auto px-7 py-7">
      <div className="mb-5 flex">
        <LibrarySearchInput
          value={q}
          label={album !== undefined ? "Search this album" : "Search albums"}
          onChange={(value) =>
            void navigate({
              search: (prev) => ({ ...prev, q: value || undefined }),
              replace: true,
            })
          }
        />
      </div>
      {album !== undefined ? (
        <>
          <Link to="/albums" search={{}} className="text-sm text-primary">
            ‹ All albums
          </Link>
          <AlbumHeader album={album} artist={album ? (artist ?? "") : artist} />
          <CollectionSongs
            query={q}
            filters={{
              ...emptySearchFilters,
              exactAlbum: album,
              exactArtist: album ? (artist ?? "") : artist,
            }}
          />
        </>
      ) : (
        <>
          <h1 className="text-3xl font-bold">Your library</h1>
          <CollectionGrid query={q} kind="albums" />
        </>
      )}
    </main>
  );
}
