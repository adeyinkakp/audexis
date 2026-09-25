import { createFileRoute } from "@tanstack/react-router";
import { Search, SlidersHorizontal, X } from "lucide-react";
import { useState } from "react";
import { AdvancedSearch } from "../../components/search/AdvancedSearch";
import { emptySearchFilters } from "../../hooks/useSearchMedia";
import { CollectionGrid } from "../../components/library/CollectionGrid";
import { CollectionSongs } from "../../components/library/CollectionSongs";
export const Route = createFileRoute("/_noneditor/search")({
  component: SearchPage,
});
function SearchPage() {
  const [text, setText] = useState("");
  const [advanced, setAdvanced] = useState(false);
  const [filters, setFilters] = useState(emptySearchFilters);
  const hasFilters = Object.values(filters).some((value) => value?.trim());
  const searching = !!text.trim() || hasFilters;
  return (
    <main
      className="h-[calc(100dvh-5.5rem)] overflow-auto px-7 pb-10"
      aria-label="Search library"
    >
      <header className="sticky top-0 z-10 flex items-center gap-3 bg-background/95 py-5 backdrop-blur">
        <div className="relative flex-1">
          <Search
            size={20}
            className="absolute left-4 top-3.5 text-muted-foreground"
          />
          <input
            autoFocus
            type="search"
            aria-label="Search your library"
            maxLength={500}
            value={text}
            onChange={(event) => setText(event.target.value)}
            placeholder="Search your library"
            className="h-12 w-full rounded-full border border-border bg-muted/20 pl-12 pr-12 outline-none focus:border-primary"
          />
          {text && (
            <button
              aria-label="Clear search"
              onClick={() => setText("")}
              className="absolute right-4 top-4"
            >
              <X size={16} />
            </button>
          )}
        </div>
        <button
          aria-expanded={advanced}
          aria-controls="advanced-search"
          onClick={() => setAdvanced(!advanced)}
          className="flex h-12 items-center gap-2 rounded-full border border-border px-4 text-sm hover:bg-muted"
        >
          <SlidersHorizontal size={16} />
          Advanced
        </button>
        <span className="hidden rounded-full bg-muted px-4 py-3 text-sm sm:block">
          Your Library
        </span>
      </header>
      {advanced && <AdvancedSearch value={filters} onChange={setFilters} />}
      {hasFilters && (
        <div className="flex justify-between py-3 text-sm text-muted-foreground">
          <span>Advanced filters apply to songs.</span>
          <button
            className="text-primary"
            onClick={() => setFilters(emptySearchFilters)}
          >
            Clear filters
          </button>
        </div>
      )}
      {!searching ? (
        <div className="py-24 text-center">
          <Search className="mx-auto mb-5 text-primary" size={40} />
          <h1 className="text-3xl font-semibold">Search your library</h1>
          <p className="mt-3 text-muted-foreground">
            Find songs, albums, artists, and playlists.
          </p>
          <p className="mt-2 text-sm text-muted-foreground">
            Title, artist, file name, and album matches come first.
          </p>
        </div>
      ) : (
        <>
          {text.trim() && <CollectionGrid preview kind="albums" query={text} />}
          <CollectionSongs preview query={text} filters={filters} />
          {text.trim() && (
            <>
              <CollectionGrid preview kind="artists" query={text} />
              <CollectionGrid preview kind="playlists" query={text} />
            </>
          )}
        </>
      )}
    </main>
  );
}
