import { useState } from "react";
import toast from "react-hot-toast";
import { useDetailsFields } from "../../hooks/useDetailsFields";
import { useMetadataFieldCatalog } from "../../hooks/useMetadataFieldCatalog";

export function MetadataFields() {
  const selected = useDetailsFields();
  const catalog = useMetadataFieldCatalog();
  const [search, setSearch] = useState("");
  if (selected.isPending || catalog.isPending) return <p>Loading fields…</p>;
  if (selected.isError || catalog.isError)
    return (
      <div role="alert">
        Could not load metadata fields.{" "}
        <button
          onClick={() => {
            void selected.refetch();
            void catalog.refetch();
          }}
        >
          Retry
        </button>
      </div>
    );
  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">
        Choose fields to show in Track Info’s Details tab.
      </p>
      <input
        aria-label="Find metadata fields"
        placeholder="Find a field…"
        value={search}
        onChange={(event) => setSearch(event.target.value)}
        className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm"
      />
      <fieldset
        disabled={selected.saving}
        className="max-h-80 space-y-1 overflow-y-auto"
      >
        {catalog.data?.fields
          .filter(
            (field) =>
              field.editable &&
              `${field.label} ${field.storageKey}`
                .toLowerCase()
                .includes(search.toLowerCase()),
          )
          .sort((a, b) => a.label.localeCompare(b.label))
          .map((field) => (
            <label
              key={field.key}
              className="flex items-center gap-3 rounded-lg px-2 py-2 text-sm hover:bg-muted"
            >
              <input
                type="checkbox"
                checked={selected.fields.includes(field.key)}
                onChange={(event) => {
                  void selected
                    .save(
                      event.target.checked
                        ? [...selected.fields, field.key]
                        : selected.fields.filter((key) => key !== field.key),
                    )
                    .catch((error) => toast.error(String(error)));
                }}
              />
              {field.label}
            </label>
          ))}
      </fieldset>
    </div>
  );
}
