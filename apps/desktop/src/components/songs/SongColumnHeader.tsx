import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical } from "lucide-react";
import { useRef } from "react";
import type { SongColumnId } from "./columns";
import { MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH } from "./columns";

type Props = {
  id: SongColumnId;
  label: string;
  width: number;
  ready: boolean;
  sorted: false | "asc" | "desc";
  onSort: () => void;
  onResize: (width: number, save?: boolean) => void;
  onResizeEnd: () => void;
};
export function SongColumnHeader({
  id,
  label,
  width,
  ready,
  sorted,
  onSort,
  onResize,
  onResizeEnd,
}: Props) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id, disabled: !ready });
  const resizing = useRef<{ x: number; width: number } | null>(null);
  return (
    <div
      ref={setNodeRef}
      role="columnheader"
      aria-sort={
        sorted === "asc"
          ? "ascending"
          : sorted === "desc"
            ? "descending"
            : "none"
      }
      className={`relative flex h-10 min-w-0 items-center border-r border-border bg-muted text-xs font-semibold text-muted-foreground ${isDragging ? "z-20 shadow-lg opacity-80" : ""}`}
      style={{ transform: CSS.Translate.toString(transform), transition }}
    >
      <button
        type="button"
        aria-label={`Move ${label} column`}
        disabled={!ready}
        {...attributes}
        {...listeners}
        className="ml-1 shrink-0 cursor-grab touch-none rounded p-1 text-muted-foreground/50 hover:text-foreground focus-visible:outline-2 focus-visible:outline-primary active:cursor-grabbing"
      >
        <GripVertical size={12} />
      </button>
      <button
        type="button"
        className="flex min-w-0 flex-1 items-center gap-2 overflow-hidden py-2 pr-3 text-left hover:text-foreground"
        onClick={onSort}
      >
        <span className="truncate">{label}</span>
        <span aria-hidden="true">
          {sorted === "asc" ? "▲" : sorted === "desc" ? "▼" : ""}
        </span>
      </button>
      <div
        role="separator"
        tabIndex={ready ? 0 : -1}
        aria-label={`Resize ${label} column`}
        aria-orientation="vertical"
        aria-valuemin={MIN_COLUMN_WIDTH}
        aria-valuemax={MAX_COLUMN_WIDTH}
        aria-valuenow={Math.round(width)}
        className="absolute inset-y-0 -right-1 z-30 w-2 cursor-col-resize touch-none hover:bg-primary/30 focus-visible:bg-primary/40 focus-visible:outline-none"
        onPointerDown={(event) => {
          if (!ready || event.button !== 0) return;
          event.preventDefault();
          event.stopPropagation();
          event.currentTarget.setPointerCapture(event.pointerId);
          resizing.current = { x: event.clientX, width };
        }}
        onPointerMove={(event) => {
          if (resizing.current)
            onResize(
              resizing.current.width + event.clientX - resizing.current.x,
            );
        }}
        onPointerUp={(event) => {
          if (!resizing.current) return;
          resizing.current = null;
          event.currentTarget.releasePointerCapture(event.pointerId);
          onResizeEnd();
        }}
        onLostPointerCapture={() => {
          if (resizing.current) {
            resizing.current = null;
            onResizeEnd();
          }
        }}
        onKeyDown={(event) => {
          if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
            event.preventDefault();
            event.stopPropagation();
            onResize(width + (event.key === "ArrowRight" ? 16 : -16), true);
          }
        }}
      />
    </div>
  );
}
