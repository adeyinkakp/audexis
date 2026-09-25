import type { SearchFilters } from "../../hooks/useSearchMedia";
const fields: {
  key: keyof SearchFilters;
  label: string;
  placeholder: string;
}[] = [
  { key: "title", label: "Title", placeholder: "Song title" },
  { key: "artist", label: "Artist", placeholder: "Artist name" },
  { key: "album", label: "Album", placeholder: "Album name" },
  { key: "fileName", label: "File name", placeholder: "Part of a file name" },
  { key: "genre", label: "Genre", placeholder: "Jazz, rock…" },
  { key: "format", label: "Format / tag type", placeholder: "FLAC, ID3…" },
  {
    key: "folder",
    label: "File path",
    placeholder: "Folder or path contains…",
  },
];
export function AdvancedSearch({
  value,
  onChange,
}: {
  value: SearchFilters;
  onChange: (value: SearchFilters) => void;
}) {
  return (
    <div
      id="advanced-search"
      className="rounded-2xl border border-border bg-muted/15 p-5"
    >
      <p className="mb-4 text-xs text-muted-foreground">
        All filled fields must match. These filters also narrow the main search.
      </p>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {fields.map(({ key, label, placeholder }) => (
          <label key={key} className="space-y-1.5 text-xs font-medium">
            <span>{label}</span>
            <input
              value={value[key]}
              maxLength={500}
              onChange={(event) =>
                onChange({ ...value, [key]: event.target.value })
              }
              placeholder={placeholder}
              className="h-10 w-full rounded-lg border border-border bg-background px-3 text-sm outline-none focus:border-primary"
            />
          </label>
        ))}
        {(
          [
            { key: "minMinutes", label: "Minimum duration (minutes)" },
            { key: "maxMinutes", label: "Maximum duration (minutes)" },
          ] as const
        ).map(({ key, label }) => (
          <label key={key} className="space-y-1.5 text-xs font-medium">
            <span>{label}</span>
            <input
              type="number"
              min="0"
              max="1000000"
              step="0.1"
              value={value[key]}
              onChange={(event) =>
                onChange({ ...value, [key]: event.target.value })
              }
              placeholder="Any"
              className="h-10 w-full rounded-lg border border-border bg-background px-3 text-sm outline-none focus:border-primary"
            />
          </label>
        ))}
      </div>
    </div>
  );
}
