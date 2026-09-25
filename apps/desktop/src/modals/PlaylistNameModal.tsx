import { useRef, useState } from "react";
import { ListMusic } from "lucide-react";
import { Modal } from "../components/Modal";
export function PlaylistNameModal({
  initialName = "",
  editing = false,
  onSave,
  onClose,
}: {
  initialName?: string;
  editing?: boolean;
  onSave: (name: string) => Promise<unknown>;
  onClose: () => void;
}) {
  const [name, setName] = useState(initialName);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const saving = useRef(false);
  const close = () => {
    if (!saving.current) onClose();
  };
  return (
    <Modal
      open
      onClose={close}
      title={editing ? "Edit Playlist" : "New Playlist"}
      description="Give your playlist a name."
      initialFocusRef={input}
      panelClassName="max-w-md rounded-2xl"
      closeOnEsc={!busy}
      closeOnOverlayClick={!busy}
    >
      <form
        className="p-6"
        onSubmit={async (event) => {
          event.preventDefault();
          if (!name.trim() || saving.current) return;
          saving.current = true;
          setBusy(true);
          setError(null);
          try {
            await onSave(name.trim());
            onClose();
          } catch (error) {
            setError(String(error));
          } finally {
            saving.current = false;
            setBusy(false);
          }
        }}
      >
        <div className="mb-6 flex h-24 w-24 items-center justify-center rounded-2xl bg-gradient-to-br from-primary/25 to-muted text-primary">
          <ListMusic size={42} />
        </div>
        <label
          htmlFor="playlist-name"
          className="mb-2 block text-sm font-medium"
        >
          Playlist name
        </label>
        <input
          ref={input}
          id="playlist-name"
          value={name}
          onChange={(event) => setName(event.target.value)}
          disabled={busy}
          required
          maxLength={500}
          placeholder="My playlist"
          className="h-12 w-full rounded-xl border border-border bg-muted/20 px-4 outline-none focus:border-primary focus:ring-2 focus:ring-primary/15"
        />
        {error && (
          <p role="alert" className="mt-3 text-sm text-destructive">
            {error}
          </p>
        )}
        <div className="mt-7 flex justify-end gap-3">
          <button
            type="button"
            onClick={close}
            disabled={busy}
            className="rounded-full px-5 py-2.5 text-sm hover:bg-muted disabled:opacity-50"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={busy || !name.trim()}
            className="rounded-full bg-primary px-5 py-2.5 text-sm font-medium text-primary-foreground disabled:opacity-50"
          >
            {busy ? "Saving…" : editing ? "Save Changes" : "Create Playlist"}
          </button>
        </div>
      </form>
    </Modal>
  );
}
