import { useQuery } from "@tanstack/react-query";
import { Search, X } from "lucide-react";
import { useId, useState, type ReactNode } from "react";
import { cn } from "../../utils";

export default function OnlineLookup<T>({
  heading,
  description,
  initialSearch,
  provider,
  disabled,
  search,
  getId,
  renderResult,
  renderPreview,
  onClose,
}: {
  heading: string;
  description: string;
  initialSearch: string;
  provider: string;
  disabled: boolean;
  search: (term: string, signal: AbortSignal) => Promise<T[]>;
  getId: (result: T) => string | number;
  renderResult: (result: T) => ReactNode;
  renderPreview: (result: T) => ReactNode;
  onClose: () => void;
}) {
  const inputId = useId();
  const [term, setTerm] = useState(initialSearch);
  const [submitted, setSubmitted] = useState("");
  const [selectedId, setSelectedId] = useState<string | number | null>(null);
  const query = useQuery({
    queryKey: ["online-search", provider, submitted],
    queryFn: ({ signal }) => search(submitted, signal),
    enabled: !!submitted,
    retry: false,
    staleTime: 5 * 60_000,
    refetchOnWindowFocus: false,
  });
  const selected = query.data?.find((result) => getId(result) === selectedId);
  return (
    <section
      aria-label={heading}
      className="rounded-xl border border-border bg-muted/30 p-4"
    >
      <div className="mb-3 flex items-start justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold">{heading}</h3>
          <p className="mt-1 text-xs text-muted-foreground">{description}</p>
        </div>
        <button
          type="button"
          aria-label={`Close ${heading}`}
          onClick={onClose}
          disabled={disabled}
          className="rounded-lg p-1 text-muted-foreground hover:bg-muted"
        >
          <X size={16} />
        </button>
      </div>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          const value = term.trim();
          if (!value || disabled || query.isFetching) return;
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
            value={term}
            disabled={disabled}
            onChange={(event) => setTerm(event.target.value)}
            placeholder="Song title and artist"
            className="mt-1 h-9 w-full rounded-lg border border-border bg-background px-3 text-sm outline-none focus:border-primary"
          />
        </div>
        <button
          type="submit"
          disabled={!term.trim() || disabled || query.isFetching}
          className="inline-flex h-9 items-center gap-2 rounded-lg bg-primary px-3 text-sm text-primary-foreground disabled:opacity-40"
        >
          <Search size={14} />
          {query.isFetching ? "Searching…" : "Search"}
        </button>
      </form>
      {query.isFetching && (
        <p role="status" className="mt-3 text-xs text-muted-foreground">
          Searching {provider}…
        </p>
      )}
      {query.isError && (
        <div role="alert" className="mt-3 text-sm text-destructive">
          Search failed:{" "}
          {query.error instanceof Error
            ? query.error.message
            : "Please try again."}{" "}
          <button
            type="button"
            onClick={() => void query.refetch()}
            disabled={disabled || query.isFetching}
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
            aria-label={`${provider} search results`}
            className="max-h-64 space-y-1 overflow-y-auto rounded-lg border border-border bg-background p-1"
          >
            {query.data.map((result) => (
              <button
                key={getId(result)}
                type="button"
                disabled={disabled}
                aria-pressed={selectedId === getId(result)}
                onClick={() => setSelectedId(getId(result))}
                className={cn(
                  "block w-full rounded-md p-3 text-left hover:bg-muted",
                  selectedId === getId(result) &&
                    "bg-muted ring-1 ring-inset ring-primary/40",
                )}
              >
                {renderResult(result)}
              </button>
            ))}
          </div>
          <div className="flex min-w-0 flex-col gap-3">
            {selected ? (
              renderPreview(selected)
            ) : (
              <p className="p-3 text-sm text-muted-foreground">
                Choose a result to preview it.
              </p>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
