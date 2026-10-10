import { useState } from "react";
import toast from "react-hot-toast";
import { Modal } from "../components/Modal";
import { useCustomFieldDefinitions } from "../hooks/useCustomFieldDefinitions";
import {
  validateDefinitions,
  type CustomFieldDefinition,
} from "../utils/customFields";

export type NewFieldDefinition = Pick<CustomFieldDefinition, "key" | "kind">;
const inputClass =
  "mt-1 w-full rounded-lg border border-border bg-background px-3 py-2 text-sm";
const buttonClass =
  "rounded-lg border border-border px-3 py-2 text-sm disabled:opacity-40";

export default function CustomFieldDefinitionsModal({
  onClose,
  initial,
}: {
  onClose: () => void;
  initial?: NewFieldDefinition;
}) {
  return (
    <Modal
      open
      onClose={onClose}
      title="Saved Custom Fields"
      panelClassName="max-w-3xl"
    >
      <CustomFieldSettings initial={initial} onClose={onClose} />
    </Modal>
  );
}

export function CustomFieldSettings({
  initial,
  onClose,
}: {
  initial?: NewFieldDefinition;
  onClose: () => void;
}) {
  const saved = useCustomFieldDefinitions();
  return saved.isPending ? (
    <p>Loading saved fields…</p>
  ) : saved.isError ? (
    <div role="alert">
      <p>Could not load saved fields.</p>
      <button className={buttonClass} onClick={() => void saved.refetch()}>
        Retry
      </button>
    </div>
  ) : (
    <DefinitionsForm
      key={JSON.stringify(initial ?? null)}
      initial={initial}
      definitions={saved.definitions}
      save={saved.save}
      onClose={onClose}
    />
  );
}

function DefinitionsForm({
  initial,
  definitions,
  save,
  onClose,
}: {
  initial?: NewFieldDefinition;
  definitions: CustomFieldDefinition[];
  save: (
    definitions: CustomFieldDefinition[],
  ) => Promise<CustomFieldDefinition[]>;
  onClose: () => void;
}) {
  const [draft, setDraft] = useState(() =>
    initial &&
    !definitions.some(
      (field) => field.key === initial.key && field.kind === initial.kind,
    )
      ? [
          ...definitions,
          { id: crypto.randomUUID(), title: initial.key, ...initial },
        ]
      : definitions,
  );
  const [saving, setSaving] = useState(false);
  const error = validateDefinitions(draft);
  const update = (id: string, change: Partial<CustomFieldDefinition>) =>
    setDraft((current) =>
      current.map((field) =>
        field.id === id ? { ...field, ...change } : field,
      ),
    );
  return (
    <div className="space-y-4">
      <fieldset disabled={saving} className="space-y-3">
        {draft.map((field) => (
          <div
            key={field.id}
            className="grid gap-3 rounded-xl border border-border p-4 sm:grid-cols-2"
          >
            <label className="text-xs">
              Display title
              <input
                className={inputClass}
                value={field.title}
                placeholder="My Mood"
                onChange={(event) =>
                  update(field.id, { title: event.target.value })
                }
              />
            </label>
            <label className="text-xs">
              Metadata key
              <input
                className={inputClass}
                value={field.key}
                placeholder="MOOD"
                onChange={(event) =>
                  update(field.id, { key: event.target.value })
                }
              />
            </label>
            <label className="text-xs">
              Field type
              <select
                className={inputClass}
                value={field.kind}
                onChange={(event) =>
                  update(field.id, {
                    kind: event.target.value as CustomFieldDefinition["kind"],
                  })
                }
              >
                <option value="text">Text</option>
                <option value="url">URL</option>
              </select>
            </label>
            <button
              type="button"
              className={`${buttonClass} self-end justify-self-end`}
              onClick={() =>
                setDraft(draft.filter((item) => item.id !== field.id))
              }
            >
              Remove definition
            </button>
          </div>
        ))}
        <button
          type="button"
          className={buttonClass}
          onClick={() =>
            setDraft([
              ...draft,
              { id: crypto.randomUUID(), title: "", key: "", kind: "text" },
            ])
          }
        >
          New Field
        </button>
      </fieldset>
      <p className="text-xs text-muted-foreground">
        Saved fields are available in Get Info and the Songs header’s
        right-click menu. Changing or removing a definition does not rewrite
        existing song tags.
      </p>
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
      <div className="flex justify-end gap-2">
        <button className={buttonClass} disabled={saving} onClick={onClose}>
          Cancel
        </button>
        <button
          className="rounded-lg bg-primary px-4 py-2 text-sm text-primary-foreground disabled:opacity-40"
          disabled={saving || !!error}
          onClick={async () => {
            setSaving(true);
            try {
              await save(draft);
              toast.success("Field definitions saved");
              onClose();
            } catch (error) {
              toast.error(String(error));
            } finally {
              setSaving(false);
            }
          }}
        >
          {saving ? "Saving…" : "Save Fields"}
        </button>
      </div>
    </div>
  );
}
