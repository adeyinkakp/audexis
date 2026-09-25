import { createFileRoute, useNavigate } from "@tanstack/react-router";
import PlaylistEditor from "../../components/PlaylistEditor";
import { LibrarySearchInput } from "../../components/library/LibrarySearchInput";

export const Route = createFileRoute("/_noneditor/playlists/$playlistId")({
  validateSearch: (search: Record<string, unknown>): { q?: string } => ({
    q: typeof search.q === "string" ? search.q : undefined,
  }),
  component: PlaylistPage,
});

function PlaylistPage() {
  const { playlistId } = Route.useParams();
  const navigate = useNavigate();
  const numericPlaylistId = Number(playlistId);
  const { q = "" } = Route.useSearch();

  return (
    <main className="min-h-screen px-8 pb-28 pt-16">
      <div className="mb-6 flex">
        <LibrarySearchInput
          value={q}
          label="Search this playlist"
          onChange={(value) =>
            void navigate({
              to: "/playlists/$playlistId",
              params: { playlistId },
              search: { q: value || undefined },
              replace: true,
            })
          }
        />
      </div>
      <PlaylistEditor
        search={q}
        playlistId={numericPlaylistId}
        onDeleted={() => {
          void navigate({ to: "/playlists" });
        }}
      />
    </main>
  );
}
