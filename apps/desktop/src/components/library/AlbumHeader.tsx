import { Link } from "@tanstack/react-router";
import { useSearchMedia, emptySearchFilters } from "../../hooks/useSearchMedia";
import { Artwork } from "./CollectionGrid";
export function AlbumHeader({
  album,
  artist,
}: {
  album: string;
  artist?: string;
}) {
  const result = useSearchMedia("", {
    ...emptySearchFilters,
    exactAlbum: album,
    exactArtist: artist,
  });
  return (
    <header className="flex flex-col gap-7 py-10 sm:flex-row sm:items-end">
      <div className="w-52 shrink-0">
        <Artwork id={result.data?.pages[0]?.files[0]?.id ?? null} />
      </div>
      <div>
        <p className="mb-3 text-sm uppercase tracking-widest text-muted-foreground">
          Album
        </p>
        <h1 className="text-4xl font-bold">{album || "Uncategorized"}</h1>
        {artist ? (
          <Link
            to="/artists"
            search={{ artist }}
            className="mt-3 block text-xl text-primary"
          >
            {artist}
          </Link>
        ) : (
          <p className="mt-3 text-muted-foreground">
            {album ? "Uncategorized artist" : "Songs without an album"}
          </p>
        )}
      </div>
    </header>
  );
}
