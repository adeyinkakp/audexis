import { LibrarySearchInput } from "../../components/library/LibrarySearchInput";
import { createFileRoute, Link } from "@tanstack/react-router";
import { CollectionGrid } from "../../components/library/CollectionGrid";
import { CollectionSongs } from "../../components/library/CollectionSongs";
import { emptySearchFilters } from "../../hooks/useSearchMedia";
export const Route = createFileRoute("/_noneditor/artists")({
  validateSearch: (
    search: Record<string, unknown>,
  ): { artist?: string; q?: string } => ({
    q: typeof search.q === "string" ? search.q : undefined,
    artist: typeof search.artist === "string" ? search.artist : undefined,
  }),
  component: ArtistsPage,
});
function ArtistsPage() {
  const { artist, q = "" } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <main className="h-[calc(100dvh-5.5rem)] overflow-auto px-7 py-7">
      <div className="mb-5 flex">
        <LibrarySearchInput
          value={q}
          label={artist !== undefined ? "Search this artist" : "Search artists"}
          onChange={(value) =>
            void navigate({
              search: (prev) => ({ ...prev, q: value || undefined }),
              replace: true,
            })
          }
        />
      </div>
      {artist !== undefined ? (
        <>
          <Link to="/artists" search={{}} className="text-sm text-primary">
            ‹ All artists
          </Link>
          <header className="py-10">
            <p className="mb-3 text-sm uppercase tracking-widest text-muted-foreground">
              Artist
            </p>
            <h1 className="text-4xl font-bold">{artist || "Uncategorized"}</h1>
          </header>
          <CollectionGrid query={q} kind="albums" artist={artist} />
          <CollectionSongs
            query={q}
            filters={{ ...emptySearchFilters, exactArtist: artist }}
          />
        </>
      ) : (
        <>
          <h1 className="text-3xl font-bold">Your library</h1>
          <CollectionGrid query={q} kind="artists" />
        </>
      )}
    </main>
  );
}
