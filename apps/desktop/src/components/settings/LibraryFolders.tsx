import { invoke } from "@tauri-apps/api/core";
import { Folder, FolderPlus, X } from "lucide-react";
import { useState } from "react";
import toast from "react-hot-toast";
export function LibraryFolders({
  folders,
  onChange,
  disabled = false,
}: {
  folders: string[];
  onChange: (folders: string[]) => void;
  disabled?: boolean;
}) {
  const [picking, setPicking] = useState(false);
  const choose = async () => {
    setPicking(true);
    try {
      const selected = await invoke<string[]>("import_roots");
      onChange([...new Set([...folders, ...selected])]);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setPicking(false);
    }
  };
  return (
    <div className="space-y-4">
      <button
        type="button"
        disabled={disabled || picking}
        onClick={() => void choose()}
        className="flex w-full items-center gap-4 rounded-2xl border border-dashed border-primary/40 bg-primary/5 p-5 text-left transition-colors hover:bg-primary/10 disabled:opacity-50"
      >
        <span className="flex size-11 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary">
          <FolderPlus size={23} />
        </span>
        <span>
          <span className="block text-sm font-semibold">
            {picking ? "Choosing folders…" : "Choose music folders"}
          </span>
          <span className="mt-1 block text-xs text-muted-foreground">
            Subfolders are included automatically.
          </span>
        </span>
      </button>
      <div className="max-h-48 space-y-2 overflow-auto">
        {folders.map((folder) => (
          <div
            key={folder}
            className="flex items-center gap-3 rounded-xl border border-border bg-muted/15 px-3 py-3"
          >
            <Folder size={17} className="shrink-0 text-primary" />
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium">
                {folder.split(/[\\/]/).filter(Boolean).pop()}
              </p>
              <p
                className="truncate text-xs text-muted-foreground"
                title={folder}
              >
                {folder}
              </p>
            </div>
            <button
              type="button"
              disabled={disabled}
              aria-label={`Remove folder ${folder}`}
              onClick={() =>
                onChange(folders.filter((item) => item !== folder))
              }
              className="rounded-lg p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
            >
              <X size={15} />
            </button>
          </div>
        ))}
      </div>
      <p className="text-xs leading-relaxed text-muted-foreground">
        Audexis reads music where it lives. Your original audio files are never
        moved or deleted by library setup.
      </p>
    </div>
  );
}
