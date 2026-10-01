import { X } from "lucide-react";
import { Modal } from "../components/Modal";
import DetailsTab from "../components/info/DetailsTab";
import { useEffect, useState } from "react";
import { cn } from "../utils";
import { useMediaFiles } from "../hooks/useMediaFiles";
import { FilesResponse } from "../hooks/useFileWatcher";

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
    {
      title: "Details",
      id: "details",
      Comp: DetailsTab,
    },
    {
      title: "Artwork",
      id: "artork",
      Comp: DetailsTab,
    },
  ];
  const [currentTabId, setCurrentTabId] = useState("details");
  const [currentTab, setCurrentTab] = useState(tabs[0]);

  useEffect(() => {}, [f]);
  useEffect(() => {
    let currTab = tabs.find((tab) => tab.id === currentTabId);
    if (!currTab) {
      currTab = tabs[0];
    }
    setCurrentTab(currTab);
  }, [currentTabId]);

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Track Info"
      header={null}
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
          aria-label="Close settings"
          className="rounded-full p-2 text-muted-foreground hover:bg-muted"
        >
          <X size={18} />
        </button>
      </div>
      <div className="px-2 mt-2">
        <div className="flex h-8 rounded gap-2 ">
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
        {f.isLoading ? (
          <div> Loading</div>
        ) : (
          !f.isError && (
            <>
              {" "}
              <currentTab.Comp
                files={f.data}
                fileIds={fileIds}
              ></currentTab.Comp>
            </>
          )
        )}
      </div>
    </Modal>
  );
}
