import { Search, X } from "lucide-react";
export function LibrarySearchInput({
  value,
  onChange,
  label,
}: {
  value: string;
  onChange: (value: string) => void;
  label: string;
}) {
  return (
    <div className="relative ml-auto w-full max-w-xs">
      <Search
        size={16}
        className="pointer-events-none absolute left-3 top-3 text-muted-foreground"
      />
      <input
        type="search"
        aria-label={label}
        placeholder={label}
        maxLength={500}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-10 w-full rounded-full border border-border bg-muted/20 pl-9 pr-9 text-sm outline-none focus:border-primary"
      />
      {value && (
        <button
          type="button"
          aria-label="Clear search"
          onClick={() => onChange("")}
          className="absolute right-3 top-3 text-muted-foreground"
        >
          <X size={16} />
        </button>
      )}
    </div>
  );
}
