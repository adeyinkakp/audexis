import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  type ReactElement,
  type HTMLAttributes,
  cloneElement,
} from "react";
import type { MenuOptions } from "../ContextMenu";
import { SongContextMenu } from "../SongContextMenu";
import {
  selectedItems,
  selectSongSelection,
  type SongSelectionItem,
} from "../../utils/songSelection";

type Selection = {
  items: SongSelectionItem[];
  contextMenuItems?: (items: SongSelectionItem[]) => MenuOptions;
  removeSelected?: () => Promise<void>;
  disabled: boolean;
  keys: Set<string>;
  fileIds: number[];
  select: (key: string, range?: boolean, additive?: boolean) => void;
};
const SelectionContext = createContext<Selection | null>(null);

export function SongCollection({
  items,
  children,
  label = "Songs",
  hasMore = false,
  className,
  disabled = false,
  onRemoveSelected,
  contextMenuItems,
}: {
  items: SongSelectionItem[];
  children: ReactNode;
  label?: string;
  hasMore?: boolean;
  className?: string;
  disabled?: boolean;
  onRemoveSelected?: (items: SongSelectionItem[]) => Promise<void>;
  contextMenuItems?: (items: SongSelectionItem[]) => MenuOptions;
}) {
  const [keys, setKeys] = useState<Set<string>>(new Set());
  const anchor = useRef<string | null>(null);
  const available = useMemo(
    () => items.filter((item) => item.fileId > 0),
    [items],
  );
  const selected = selectedItems(available, keys);
  const selectedKeys = new Set(selected.map((item) => item.key));
  const fileIds = [...new Set(selected.map((item) => item.fileId))];
  const removeSelected = onRemoveSelected
    ? () => onRemoveSelected(selected)
    : undefined;
  useEffect(() => {
    setKeys((previous) => {
      const next = new Set(
        selectedItems(available, previous).map((item) => item.key),
      );
      return next.size === previous.size ? previous : next;
    });
  }, [available]);
  const select = (key: string, range = false, additive = false) => {
    if (disabled) return;
    const previousAnchor = anchor.current;
    setKeys((previous) =>
      selectSongSelection(
        available,
        previous,
        key,
        previousAnchor,
        range,
        additive,
      ),
    );
    if (!range) anchor.current = key;
  };
  const selectAll = () => {
    if (!disabled) setKeys(new Set(available.map((item) => item.key)));
  };
  return (
    <SelectionContext.Provider
      value={{
        keys: selectedKeys,
        fileIds,
        select,
        disabled,
        removeSelected,
        items: selected,
        contextMenuItems,
      }}
    >
      <div
        className={className}
        role="group"
        aria-label={label}
        onKeyDown={(event) => {
          const target = event.target as HTMLElement;
          if (
            disabled ||
            target.closest(
              'input, textarea, select, [contenteditable="true"], [role="menu"]',
            )
          )
            return;
          if (
            (event.metaKey || event.ctrlKey) &&
            event.key.toLowerCase() === "a"
          ) {
            event.preventDefault();
            event.stopPropagation();
            selectAll();
          } else if (event.key === "Escape" && selected.length) {
            event.preventDefault();
            event.stopPropagation();
            setKeys(new Set());
          }
        }}
      >
        {available.length > 0 && (
          <div className="flex shrink-0 flex-wrap items-center gap-3 border-b border-border/60 px-2 py-2 text-xs">
            <button
              disabled={disabled}
              className="rounded px-2 py-1 hover:bg-muted"
              onClick={selectAll}
            >
              {hasMore ? "Select all loaded" : "Select all"}
            </button>
            <span role="status" className="text-muted-foreground">
              {selected.length
                ? `${selected.length} selected`
                : hasMore
                  ? "Load more songs to include them."
                  : `${available.length} songs`}
            </span>
            {selected.length > 0 && (
              <>
                <button
                  disabled={disabled}
                  className="rounded px-2 py-1 hover:bg-muted"
                  onClick={() => setKeys(new Set())}
                >
                  Clear selection
                </button>
              </>
            )}
          </div>
        )}
        {children}
      </div>
    </SelectionContext.Provider>
  );
}

export function SongCollectionItem({
  itemKey,
  fileId,
  children,
  playlistId,
  onRemoveFromPlaylist,
  onPlay,
}: {
  itemKey: string;
  fileId: number;
  children: ReactElement<HTMLAttributes<HTMLElement>>;
  playlistId?: number;
  onRemoveFromPlaylist?: () => void | Promise<void>;
  onPlay?: () => void;
}) {
  const selection = useContext(SelectionContext);
  const selected = selection?.keys.has(itemKey) ?? false;
  const isControl = (target: EventTarget, row: HTMLElement) => {
    const control = (target as HTMLElement).closest(
      "a, button, input, select, textarea, [data-song-drag-handle]",
    );
    return control !== null && control !== row;
  };
  const element = cloneElement(children, {
    tabIndex: children.props.tabIndex ?? 0,
    "aria-description": selected
      ? "Selected song. Double-click or press Enter to play."
      : "Click to select. Double-click or press Enter to play.",
    className: `${children.props.className ?? ""} select-none outline-none focus-visible:ring-2 focus-visible:ring-primary ${selected ? "bg-primary/10 ring-1 ring-inset ring-primary/40" : ""}`,
    onClick: (event) => {
      if (isControl(event.target, event.currentTarget) || event.detail > 1)
        return;
      event.currentTarget.focus({ preventScroll: true });
      selection?.select(
        itemKey,
        event.shiftKey,
        event.ctrlKey || event.metaKey,
      );
    },
    onDoubleClick: (event) => {
      if (selection?.disabled || isControl(event.target, event.currentTarget))
        return;
      event.preventDefault();
      if (onPlay) onPlay();
      else children.props.onDoubleClick?.(event);
    },
    onContextMenu: (event) => {
      if (!selected) selection?.select(itemKey);
      children.props.onContextMenu?.(event);
    },
    onKeyDown: (event) => {
      if (selection?.disabled || event.target !== event.currentTarget) return;
      if (event.key === " ") {
        event.preventDefault();
        selection?.select(
          itemKey,
          event.shiftKey,
          event.ctrlKey || event.metaKey,
        );
      } else if (event.key === "Enter" && onPlay) {
        event.preventDefault();
        onPlay();
      } else children.props.onKeyDown?.(event);
    },
  });
  return (
    <SongContextMenu
      fileId={fileId}
      extraItems={
        selection?.contextMenuItems
          ? () =>
              selection.contextMenuItems!(
                selected ? selection.items : [{ key: itemKey, fileId }],
              )
          : undefined
      }
      fileIds={selected ? selection?.fileIds : undefined}
      playlistId={playlistId}
      onRemoveFromPlaylist={
        selected && selection && selection.keys.size > 1
          ? selection.removeSelected
          : onRemoveFromPlaylist
      }
    >
      {element}
    </SongContextMenu>
  );
}
