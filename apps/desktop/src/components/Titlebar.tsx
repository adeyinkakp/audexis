import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRouter, useRouterState } from "@tanstack/react-router";
import {
  ChevronLeft,
  ChevronRight,
  Copy,
  Minus,
  Square,
  X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { cn } from "../utils";

const isWindows = navigator.userAgent.includes("Windows");

export default function Titlebar() {
  const router = useRouter();
  const historyIndex = useRouterState({
    select: (state) => state.location.state.__TSR_index,
  });
  const furthestIndex = useRef(historyIndex);
  const [isMaximized, setIsMaximized] = useState(false);

  if (historyIndex > furthestIndex.current) {
    furthestIndex.current = historyIndex;
  }

  useEffect(() => {
    return router.history.subscribe(({ location, action }) => {
      if (action.type === "PUSH") {
        furthestIndex.current = location.state.__TSR_index;
      }
    });
  }, [router]);

  useEffect(() => {
    if (!isWindows) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;
    const appWindow = getCurrentWindow();
    const refreshMaximized = async () => {
      const maximized = await appWindow.isMaximized();
      if (!disposed) setIsMaximized(maximized);
    };

    void refreshMaximized();
    void appWindow.onResized(refreshMaximized).then((cleanup) => {
      if (disposed) cleanup();
      else unlisten = cleanup;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const canGoBack = router.history.canGoBack();
  const canGoForward = historyIndex < furthestIndex.current;
  const windowButtonClass =
    "flex h-14 w-12 items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:z-10 focus-visible:outline-2 focus-visible:outline-inset focus-visible:outline-primary";

  return (
    <div
      data-tauri-drag-region={true}
      className="fixed top-0 z-99 flex h-14 w-full select-none items-center border-b border-border bg-popover/80 backdrop-blur"
    >
      <nav
        aria-label="Page history"
        className={cn(
          "relative z-10 flex items-center gap-1",
          isWindows ? "ml-3" : "ml-20",
        )}
      >
        <button
          type="button"
          aria-label="Go back"
          title="Back"
          disabled={!canGoBack}
          onClick={() => router.history.back()}
          className="flex size-8 items-center justify-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground disabled:pointer-events-none disabled:opacity-30"
        >
          <ChevronLeft size={19} />
        </button>
        <button
          type="button"
          aria-label="Go forward"
          title="Forward"
          disabled={!canGoForward}
          onClick={() => router.history.forward()}
          className="flex size-8 items-center justify-center rounded-lg text-muted-foreground transition-colors hover:bg-muted hover:text-foreground disabled:pointer-events-none disabled:opacity-30"
        >
          <ChevronRight size={19} />
        </button>
      </nav>

      <div data-tauri-drag-region={true} className="absolute inset-0" />

      {isWindows && (
        <div className="relative z-10 ml-auto flex h-full items-stretch">
          <button
            type="button"
            aria-label="Minimize window"
            title="Minimize"
            onClick={() => void getCurrentWindow().minimize()}
            className={windowButtonClass}
          >
            <Minus size={16} />
          </button>
          <button
            type="button"
            aria-label={isMaximized ? "Restore window" : "Maximize window"}
            title={isMaximized ? "Restore" : "Maximize"}
            onClick={() => void getCurrentWindow().toggleMaximize()}
            className={windowButtonClass}
          >
            {isMaximized ? <Copy size={13} /> : <Square size={13} />}
          </button>
          <button
            type="button"
            aria-label="Close window"
            title="Close"
            onClick={() => void getCurrentWindow().close()}
            className={cn(
              windowButtonClass,
              "hover:bg-[#c42b1c] hover:text-white",
            )}
          >
            <X size={17} />
          </button>
        </div>
      )}
    </div>
  );
}
