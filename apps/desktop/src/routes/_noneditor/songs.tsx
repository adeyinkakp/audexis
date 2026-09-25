import { SongContextMenu } from "../../components/SongContextMenu";
import { HeartButton } from "../../components/HeartButton";
import { LibrarySearchInput } from "../../components/library/LibrarySearchInput";
import {
  flexRender,
  createSortedRowModel,
  rowSortingFeature,
  useTable,
} from "@tanstack/react-table";
import { useVirtualizer } from "@tanstack/react-virtual";
import { createFileRoute } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useRef } from "react";
import {
  DndContext,
  PointerSensor,
  KeyboardSensor,
  closestCenter,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  horizontalListSortingStrategy,
  sortableKeyboardCoordinates,
} from "@dnd-kit/sortable";
import { useFileWatcher } from "../../hooks/useFileWatcher";
import { useNowPlayingState } from "../../hooks/useNowPlayingState";
import { useStore } from "../../hooks/useStore";
import { useSongColumns } from "../../hooks/useSongColumns";
import { ContextMenuArea } from "../../components/ContextMenu";
import { SongColumnHeader } from "../../components/songs/SongColumnHeader";
import {
  songColumns,
  displaySongValue,
  type SongRow,
  type SongColumnId,
} from "../../components/songs/columns";

export const Route = createFileRoute("/_noneditor/songs")({
  validateSearch: (search: Record<string, unknown>): { q?: string } => ({
    q: typeof search.q === "string" ? search.q : undefined,
  }),
  component: SongsPage,
});
const alphabeticalColumns = [...songColumns].sort((a, b) =>
  a.label.localeCompare(b.label),
);

function SongsPage() {
  const { q = "" } = Route.useSearch();
  const navigate = Route.useNavigate();
  const {
    data,
    isPending,
    isError,
    hasNextPage,
    isFetchingNextPage,
    fetchNextPage,
  } = useFileWatcher(q);
  const scrollRef = useRef<HTMLDivElement>(null);
  const { currentTrackId, song } = useNowPlayingState();
  const layout = useSongColumns();
  const { preferences } = useStore();
  const rowHeight =
    preferences.density === "compact"
      ? 34
      : preferences.density === "comfort"
        ? 56
        : 46;
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );
  const rows = useMemo<SongRow[]>(() => {
    if (!data) return [];
    return data.allIds.map((id) => {
      const file = data.filesById[id];
      const metadata = data.metadataByFileId[id] ?? [];
      const valueFor = (key: string) =>
        metadata.find((item) => item.key === key)?.value ?? "";
      return {
        ...file,
        title: valueFor("title"),
        artist: valueFor("artist"),
        album: valueFor("album"),
        genre: valueFor("genre"),
      };
    });
  }, [data]);
  const columns = useMemo(
    () =>
      layout.order.map((id) => ({
        accessorKey: id,
        header: songColumns.find((column) => column.id === id)!.label,
        cell: ({ getValue }: { getValue: () => unknown }) =>
          displaySongValue(id, getValue()),
      })),
    [layout.order],
  );
  const table = useTable({
    data: rows,
    columns,
    features: { rowSortingFeature, sortedRowModel: createSortedRowModel() },
  });
  const rowVirtualizer = useVirtualizer({
    count: rows.length + (hasNextPage ? 1 : 0),
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight,
    scrollMargin: 40,
    overscan: 12,
  });
  useEffect(() => {
    rowVirtualizer.measure();
  }, [rowHeight, rowVirtualizer]);
  const virtualRows = rowVirtualizer.getVirtualItems();
  const lastVisibleIndex = virtualRows[virtualRows.length - 1]?.index ?? -1;
  useEffect(() => {
    if (
      lastVisibleIndex >= rows.length - 10 &&
      hasNextPage &&
      !isFetchingNextPage
    )
      void fetchNextPage();
  }, [
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    lastVisibleIndex,
    rows.length,
  ]);

  const widths = layout.order.map(
    (id) =>
      layout.widths[id] ??
      songColumns.find((column) => column.id === id)!.width,
  );
  const gridTemplateColumns = widths.map((width) => `${width}px`).join(" ");
  const tableWidth = widths.reduce((total, width) => total + width, 0);
  const menuItems = () =>
    alphabeticalColumns.map((column) => ({
      text: column.label,
      checked: layout.order.includes(column.id),
      disabled: !layout.ready,
      action: () => layout.toggle(column.id),
    }));

  return (
    <main
      className="flex flex-col h-[calc(100dvh-5.5rem)] min-h-0 w-full overflow-hidden"
      aria-label="Songs"
    >
      <header className="flex shrink-0 items-center gap-4 px-5 py-4">
        <h1 className="text-2xl font-semibold">Songs</h1>
        <LibrarySearchInput
          value={q}
          label="Search songs"
          onChange={(value) => {
            scrollRef.current?.scrollTo({ top: 0 });
            void navigate({ search: { q: value || undefined }, replace: true });
          }}
        />
      </header>
      <div
        ref={scrollRef}
        className="min-h-0 flex-1 w-full overflow-auto overscroll-contain"
      >
        <div
          className="flex min-w-full items-start"
          style={{ width: tableWidth + 44 }}
        >
          <div
            aria-label="Song favorites"
            className="sticky left-0 z-30 w-11 shrink-0 self-stretch border-r border-border bg-background"
          >
            <div className="sticky top-0 z-10 h-10 border-b border-border bg-muted" />
            <div
              className="relative"
              style={{ height: rowVirtualizer.getTotalSize() }}
            >
              {virtualRows.map((virtualRow) => {
                const row = table.getRowModel().rows[virtualRow.index];
                if (!row) return null;
                return (
                  <div
                    key={row.original.id}
                    className="absolute left-0 flex w-full items-center justify-center border-b border-border/70"
                    style={{
                      height: virtualRow.size,
                      transform: `translateY(${virtualRow.start - 40}px)`,
                    }}
                  >
                    <HeartButton fileId={row.original.id} />
                  </div>
                );
              })}
            </div>
          </div>
          <div
            role="table"
            aria-label="Songs"
            aria-rowcount={rows.length}
            aria-colcount={layout.order.length}
            className="min-w-0 flex-1 text-left text-sm"
            style={{ width: tableWidth || "100%" }}
          >
            <div
              role="rowgroup"
              className="sticky top-0 z-20 min-w-full border-b border-border bg-muted"
            >
              <ContextMenuArea items={menuItems}>
                <DndContext
                  sensors={sensors}
                  collisionDetection={closestCenter}
                  onDragEnd={({ active, over }) => {
                    if (!over || active.id === over.id) return;
                    const from = layout.order.indexOf(
                      active.id as SongColumnId,
                    );
                    const to = layout.order.indexOf(over.id as SongColumnId);
                    if (from >= 0 && to >= 0)
                      layout.reorder(arrayMove(layout.order, from, to));
                  }}
                >
                  <SortableContext
                    items={layout.order}
                    strategy={horizontalListSortingStrategy}
                  >
                    <div
                      role="row"
                      className="grid h-10"
                      style={{ gridTemplateColumns }}
                    >
                      {table
                        .getHeaderGroups()[0]
                        ?.headers.map((header, index) => (
                          <SongColumnHeader
                            key={header.id}
                            id={header.column.id as SongColumnId}
                            label={String(header.column.columnDef.header)}
                            width={widths[index]}
                            ready={layout.ready}
                            sorted={header.column.getIsSorted()}
                            onSort={() => header.column.toggleSorting()}
                            onResize={(width, save) =>
                              layout.resize(
                                header.column.id as SongColumnId,
                                width,
                                save,
                              )
                            }
                            onResizeEnd={layout.persist}
                          />
                        ))}
                      {layout.order.length === 0 && (
                        <div className="px-4 py-2 text-xs text-muted-foreground">
                          Right-click here to show columns
                        </div>
                      )}
                    </div>
                  </SortableContext>
                </DndContext>
              </ContextMenuArea>
            </div>
            <div
              role="rowgroup"
              className="relative"
              style={{ height: rowVirtualizer.getTotalSize() }}
            >
              {isPending && (
                <div className="px-4 py-8 text-muted-foreground">
                  Loading songs...
                </div>
              )}
              {isError && (
                <div className="px-4 py-8 text-destructive">
                  Could not load songs.
                </div>
              )}
              {!isPending && !isError && rows.length === 0 && (
                <div className="px-4 py-8 text-muted-foreground">
                  No songs found.
                </div>
              )}
              {virtualRows.map((virtualRow) => {
                const row = table.getRowModel().rows[virtualRow.index];
                if (!row)
                  return (
                    <div
                      key="loading"
                      className="absolute left-0 px-4 py-3 text-muted-foreground"
                      style={{
                        transform: `translateY(${virtualRow.start - 40}px)`,
                      }}
                    >
                      Loading more songs...
                    </div>
                  );
                const isCurrent =
                  row.original.id === currentTrackId ||
                  (!!song?.path && song.path === row.original.path);
                return (
                  <SongContextMenu fileId={row.original.id} key={row.id}>
                    <div
                      role="row"
                      className={`absolute left-0 grid min-w-full border-b border-border/70 transition-colors hover:bg-muted/30 ${isCurrent ? "bg-primary/8 text-primary" : ""}`}
                      style={{
                        gridTemplateColumns,
                        height: virtualRow.size,
                        transform: `translateY(${virtualRow.start - 40}px)`,
                      }}
                      onClick={() => {
                        const sortedRows = table.getRowModel().rows;
                        const queue = sortedRows.map(({ original }) => ({
                          path: original.path,
                          id: original.id,
                          occurrence: null,
                        }));
                        const currentIndex = sortedRows.findIndex(
                          (item) => item.original.id === row.original.id,
                        );
                        void invoke("play_song", {
                          playbackInfo: {
                            curr: queue[currentIndex],
                            queue,
                            currentIndex,
                          },
                        }).catch((error) =>
                          console.error("Could not play song", error),
                        );
                      }}
                    >
                      {row.getAllCells().map((cell) => (
                        <div
                          role="cell"
                          key={cell.id}
                          className="flex min-w-0 items-center self-center px-4"
                          title={displaySongValue(
                            cell.column.id as SongColumnId,
                            cell.getValue(),
                          )}
                        >
                          <span className="truncate">
                            {flexRender(
                              cell.column.columnDef.cell,
                              cell.getContext(),
                            )}
                          </span>
                        </div>
                      ))}
                    </div>
                  </SongContextMenu>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}
