import { X } from "lucide-react";
import { Modal } from "../components/Modal";
import DetailsTab from "../components/info/DetailsTab";
import LyricsTab from "../components/info/LyricsTab";
import ArtworkTab from "../components/info/ArtworkTab";
import { useEffect, useState } from "react";
import { cn } from "../utils";
import { useMediaFiles } from "../hooks/useMediaFiles";

export default function TrackInfoModal({
  open,
  onClose,
  fileIds,
}: {
  open: boolean;
  onClose: () => void;
  fileIds: number[];
}) {
  const f = useMediaFiles(fileIds);
  const tabs = [
    { title: "Details", id: "details" },
    { title: "Artwork", id: "artwork" },
    { title: "Lyrics", id: "lyrics" },
    { title: "Synchronized Lyrics", id: "synced" },
  ] as const;
  const [currentTabId, setCurrentTabId] = useState("details");

  useEffect(() => {
    if (open) setCurrentTabId("details");
  }, [open]);

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Track Info"
      header={null}
      sizeMax
      panelClassName="max-w-5xl h-[80vh] rounded-3xl bg-background"
      bodyClassName="p-0"
    >
      <div className="flex items-center justify-between border-b border-border px-7 py-5">
        <div className="flex items-center gap-2.5">
          <h1 className="text-lg font-semibold">
            Track Info ({fileIds.length})
          </h1>
        </div>
        <button
          type="button"

          onClick={onClose}
          aria-label="Close track info"
          className="rounded-full p-2 text-muted-foreground hover:bg-muted"
        >
          <X size={18} />
        </button>
      </div>
      <div className="flex min-h-0 flex-1 flex-col px-2 pt-2">
        <div className="flex h-9 shrink-0 gap-2 rounded px-1">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              aria-label={`${tab.title} Tab`}
              onClick={() => setCurrentTabId(tab.id)}
              className={cn(
                "rounded-lg px-2 text-muted-foreground  transition-colors hover:bg-active",
                currentTabId === tab.id && "bg-primary/8 text-primary",
              )}
            >
              {tab.title}
            </button>
          ))}
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto">
          {f.isLoading && (
            <div className="flex h-64 items-center justify-center text-sm text-muted-foreground">
              Loading track info…
            </div>
          )}
          {f.isError && (
            <div className="flex h-64 items-center justify-center text-sm text-destructive">
              Track information could not be loaded.
            </div>
          )}
          {f.data && currentTabId === "details" && (
            <DetailsTab files={f.data} />
          )}
          {f.data && currentTabId === "artwork" && (
            <ArtworkTab files={f.data} />
          )}
          {f.data && open && ["lyrics", "synced"].map((tab) => (
            <div key={`${fileIds.join(",")}-${tab}`} hidden={currentTabId !== tab} className="min-h-full">
              <LyricsTab files={f.data} synchronized={tab === "synced"} active={currentTabId === tab} />
            </div>
          ))}
        </div>
      </div>
    </Modal>
  );
}
