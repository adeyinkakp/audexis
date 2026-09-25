import { createFileRoute } from "@tanstack/react-router";
import { Heart } from "lucide-react";
import { CollectionSongs } from "../../components/library/CollectionSongs";
import { LibrarySearchInput } from "../../components/library/LibrarySearchInput";
export const Route = createFileRoute("/_noneditor/favorites")({
  validateSearch: (search: Record<string, unknown>): { q?: string } => ({
    q: typeof search.q === "string" ? search.q : undefined,
  }),
  component: FavoritesPage,
});
function FavoritesPage() {
  const { q = "" } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <main className="h-[calc(100dvh-5.5rem)] overflow-auto px-7 py-7">
      <LibrarySearchInput
        value={q}
        label="Search Favorites"
        onChange={(value) =>
          void navigate({ search: { q: value || undefined }, replace: true })
        }
      />
      <header className="flex items-center gap-6 py-10">
        <div className="flex h-28 w-28 items-center justify-center rounded-2xl bg-gradient-to-br from-primary to-pink-400 text-white">
          <Heart size={48} fill="currentColor" />
        </div>
        <div>
          <p className="mb-2 text-sm text-muted-foreground">Made by you</p>
          <h1 className="text-4xl font-bold">Favorites</h1>
          <p className="mt-3 text-sm text-muted-foreground">
            Heart a song anywhere to keep it here. Unheart it to remove it.
          </p>
        </div>
      </header>
      <CollectionSongs query={q} favoritesOnly />
    </main>
  );
}
