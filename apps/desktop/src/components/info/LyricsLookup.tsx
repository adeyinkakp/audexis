import { useQuery } from "@tanstack/react-query";
import { lyricsClient } from "../../utils/lyricsClient";
import { Search, X } from "lucide-react";
import { useId, useState } from "react";
import { formatDuration } from "../../utils/duration";
import { cn } from "../../utils";

export default function LyricsLookup({
  title,
  artist,
  synchronized,
  hasDraft,
  disabled,
  onApply,
  onClose,
}: {
  title: string;
  artist: string;
  synchronized: boolean;
  hasDraft: boolean;
  disabled: boolean;
  onApply: (lyrics: string) => void;
  onClose: () => void;
}) {
  const inputId = useId();
  const [search, setSearch] = useState(
    [title, artist].filter(Boolean).join(" "),
  );
  const [submitted, setSubmitted] = useState("");
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const query = useQuery({
    queryKey: ["lrclib-search", submitted],
    queryFn: ({ signal }) =>
      lyricsClient.searchLyrics({ query: submitted }, { signal }),
    enabled: !!submitted,
    retry: false,
    staleTime: 5 * 60_000,
    refetchOnWindowFocus: false,
  });

  const selected = query.data?.find((match) => match.id === selectedId);
  const text =
    (synchronized ? selected?.syncedLyrics : selected?.plainLyrics) ?? "";
  const type = synchronized ? "synchronized" : "plain";

  return (
    <section
      aria-label="Find lyrics on LRCLIB"
      className="rounded-xl border border-border bg-muted/30 p-4"
    >
      <div className="mb-3 flex items-start justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold">Find lyrics on LRCLIB</h3>
          <p className="mt-1 text-xs text-muted-foreground">
            Search by song and artist, then choose {type} lyrics for this
            editor.
          </p>
        </div>
        <button
          type="button"
          aria-label="Close lyrics search"
          onClick={onClose}
          className="rounded-lg p-1 text-muted-foreground hover:bg-muted"
        >
          <X size={16} />
        </button>
      </div>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          const value = search.trim();
          if (!value || disabled) return;
          setSelectedId(null);
          if (value === submitted) void query.refetch();
          else setSubmitted(value);
        }}
        className="flex items-end gap-2"
      >
        <div className="min-w-0 flex-1">
          <label htmlFor={inputId} className="text-xs text-muted-foreground">
            Song and artist
          </label>
          <input
            id={inputId}
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Song title and artist"
            className="mt-1 h-9 w-full rounded-lg border border-border bg-background px-3 text-sm outline-none focus:border-primary"
          />
        </div>
        <button
          type="submit"
          disabled={!search.trim() || disabled || query.isFetching}
          className="inline-flex h-9 items-center gap-2 rounded-lg bg-primary px-3 text-sm text-primary-foreground disabled:opacity-40"
        >
          <Search size={14} />
          {query.isFetching ? "Searching…" : "Search"}
        </button>
      </form>
      {query.isFetching && (
        <p role="status" className="mt-3 text-xs text-muted-foreground">
          Searching LRCLIB…
        </p>
      )}
      {query.isError && (
        <div role="alert" className="mt-3 text-sm text-destructive">
          Lyrics search failed:{" "}
          {query.error instanceof Error
            ? query.error.message
            : "Please try again."}{" "}
          <button
            type="button"
            onClick={() => void query.refetch()}
            disabled={query.isFetching}
            className="underline"
          >
            Retry
          </button>
        </div>
      )}
      {query.isSuccess && !query.isFetching && !query.data.length && (
        <p role="status" className="mt-3 text-sm text-muted-foreground">
          No matches found. Try a shorter title or a different artist name.
        </p>
      )}
      {!!query.data?.length && !query.isFetching && !query.isError && (
        <div className="mt-3 grid gap-3 md:grid-cols-2">
          <div
            aria-label="Lyrics search results"
            className="max-h-64 space-y-1 overflow-y-auto rounded-lg border border-border bg-background p-1"
          >
            {query.data.map((match) => (
              <button
                key={match.id}
                type="button"
                aria-pressed={selectedId === match.id}
                onClick={() => setSelectedId(match.id)}
                className={cn(
                  "block w-full rounded-md p-3 text-left hover:bg-muted",
                  selectedId === match.id &&
                    "bg-muted ring-1 ring-inset ring-primary/40",
                )}
              >
                <span className="block text-sm font-medium">
                  {match.trackName}
                </span>
                <span className="mt-1 block text-xs text-muted-foreground">
                  {match.artistName} · {match.albumName || "Unknown album"} ·{" "}
                  {formatDuration(match.duration * 1000)}
                </span>
                <span className="mt-2 block text-[11px] text-muted-foreground">
                  {match.instrumental
                    ? "Instrumental"
                    : [
                        match.plainLyrics?.trim() && "Plain",
                        match.syncedLyrics?.trim() && "Synchronized",
                      ]
                        .filter(Boolean)
                        .join(" · ") || "No lyrics"}
                </span>
              </button>
            ))}
          </div>
          <div className="flex min-w-0 flex-col gap-3">
            {selected ? (
              <>
                <p className="text-xs font-medium">
                  {type === "plain"
                    ? "Plain lyrics preview"
                    : "Synchronized lyrics preview"}
                </p>
                {text.trim() ? (
                  <pre className="max-h-44 overflow-auto whitespace-pre-wrap wrap-break-word rounded-lg bg-background p-3 font-sans text-xs leading-6">
                    {text}
                  </pre>
                ) : (
                  <p className="text-sm text-muted-foreground">
                    {selected.instrumental
                      ? "This recording is marked as instrumental."
                      : `This match has no ${type} lyrics. Choose another result.`}
                  </p>
                )}
                <p className="text-xs text-muted-foreground">
                  {hasDraft ? "This replaces the text in this tab. " : ""}Review
                  it in the editor, then save it to your file.
                </p>
                <button
                  type="button"
                  disabled={!text.trim() || disabled}
                  onClick={() => onApply(text)}
                  className="h-9 rounded-lg bg-primary px-3 text-sm text-primary-foreground disabled:opacity-40"
                >
                  Use {type} lyrics
                </button>
              </>
            ) : (
              <p className="p-3 text-sm text-muted-foreground">
                Choose a result to preview its lyrics.
              </p>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
