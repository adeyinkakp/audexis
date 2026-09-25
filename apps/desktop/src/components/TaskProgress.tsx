import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { CheckCircle2, LoaderCircle, XCircle } from "lucide-react";
import { useEffect, useState } from "react";

type Task = {
  id: string;
  label: string;
  status: "running" | "completed" | "failed";
  current: number;
  total: number | null;
  message: string | null;
};

export function TaskProgress() {
  const [tasks, setTasks] = useState<Record<string, Task>>({});

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const receive = (task: Task) => {
      setTasks((current) => ({ ...current, [task.id]: task }));
      if (task.status !== "running") {
        window.setTimeout(() => {
          setTasks((current) => {
            if (current[task.id]?.status === "running") return current;
            const next = { ...current };
            delete next[task.id];
            return next;
          });
        }, 5000);
      }
    };
    void invoke<Task[]>("get_active_tasks").then((active) => {
      if (!disposed) setTasks(Object.fromEntries(active.map((task) => [task.id, task])));
    });
    void listen<Task>("task-progress", ({ payload }) => receive(payload)).then((cleanup) => {
      if (disposed) cleanup();
      else unlisten = cleanup;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const visible = Object.values(tasks);
  if (!visible.length) return null;

  return (
    <div className="fixed bottom-24 right-5 z-50 w-80 space-y-2" aria-live="polite">
      {visible.map((task) => {
        const percent = task.total ? Math.min(100, (task.current / task.total) * 100) : null;
        const Icon = task.status === "running" ? LoaderCircle : task.status === "completed" ? CheckCircle2 : XCircle;
        return (
          <div key={task.id} className="rounded-2xl border border-border bg-background/95 p-4 shadow-xl backdrop-blur">
            <div className="flex items-center gap-3">
              <Icon size={17} className={task.status === "running" ? "animate-spin text-primary" : task.status === "completed" ? "text-green-500" : "text-destructive"} />
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium">{task.label}</p>
                <p className="mt-0.5 truncate text-xs text-muted-foreground">{task.message ?? "Starting…"}</p>
              </div>
              {percent !== null && <span className="text-xs tabular-nums text-muted-foreground">{Math.round(percent)}%</span>}
            </div>
            <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-muted">
              <div className={percent === null ? "h-full w-1/3 animate-pulse rounded-full bg-primary" : "h-full rounded-full bg-primary transition-[width] duration-300"} style={percent === null ? undefined : { width: `${percent}%` }} />
            </div>
          </div>
        );
      })}
    </div>
  );
}
