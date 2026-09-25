import { invoke } from "@tauri-apps/api/core";
import { Copy, FileWarning, RefreshCw, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import toast from "react-hot-toast";
import { Modal } from "../components/Modal";
import { logError } from "../utils/logger";

type LogFile = { name: string; content: string };

export function LogsModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [files, setFiles] = useState<LogFile[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      setFiles(await invoke<LogFile[]>("get_logs"));
    } catch (error) {
      logError("Could not load logs", error);
      setLoadError(String(error));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open) void load();
  }, [open, load]);

  const allLogs = files
    .map((file) => {
      const entries = file.content.trimEnd().split(/\r?\n/).reverse().join("\n");
      return `===== ${file.name} =====\n${entries}`;
    })
    .join("\n\n");

  return (
    <Modal open={open} onClose={onClose} title="Application logs" header={null} panelClassName="max-w-5xl h-[80vh] rounded-3xl bg-background" bodyClassName="p-0">
      <div className="flex items-center justify-between border-b border-border px-7 py-5">
        <div className="flex items-center gap-2.5">
          <FileWarning size={20} className="text-primary" />
          <div>
            <h1 className="text-lg font-semibold">Application logs</h1>
            <p className="text-xs text-muted-foreground">Persisted diagnostics from Audexis</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button type="button" onClick={() => void load()} disabled={loading} aria-label="Refresh logs" className="rounded-full p-2 text-muted-foreground hover:bg-muted disabled:opacity-50">
            <RefreshCw size={18} className={loading ? "animate-spin" : ""} />
          </button>
          <button type="button" onClick={onClose} aria-label="Close logs" className="rounded-full p-2 text-muted-foreground hover:bg-muted">
            <X size={18} />
          </button>
        </div>
      </div>
      <div className="flex h-full min-h-0 flex-col p-6">
        {loadError ? (
          <div role="alert" className="rounded-xl bg-destructive/10 p-4 text-sm text-destructive">{loadError}</div>
        ) : files.length === 0 && !loading ? (
          <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">No logs have been recorded yet.</div>
        ) : (
          <pre className="min-h-0 flex-1 overflow-auto whitespace-pre-wrap break-words rounded-xl border border-border bg-muted/20 p-4 font-mono text-xs leading-relaxed text-foreground/80">{allLogs}</pre>
        )}
      </div>
      <div className="flex items-center justify-between border-t border-border px-7 py-4">
        <p className="text-xs text-muted-foreground">{files.length} log {files.length === 1 ? "file" : "files"}</p>
        <button type="button" disabled={!allLogs} onClick={() => void navigator.clipboard.writeText(allLogs).then(() => toast.success("Logs copied")).catch((error) => logError("Could not copy logs", error))} className="inline-flex items-center gap-2 rounded-full border border-border px-4 py-2 text-sm hover:bg-muted disabled:opacity-40">
          <Copy size={14} />
          Copy all
        </button>
      </div>
    </Modal>
  );
}
