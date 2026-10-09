import type { ReactNode } from "react";
import { Link } from "@tanstack/react-router";
import { Artwork } from "../library/CollectionGrid";
import { HeartButton } from "../HeartButton";
import { SongCollectionItem } from "./SongCollection";

export function SongItem({
  itemKey,
  fileId,
  title,
  artist,
  detail,
  onPlay,
  current = false,
  disabled = false,
  variant = "row",
  favorite = false,
}: {
  itemKey: string;
  fileId: number;
  title: string;
  artist?: string;
  detail?: ReactNode;
  onPlay: () => void;
  current?: boolean;
  disabled?: boolean;
  variant?: "row" | "card";
  favorite?: boolean;
}) {
  return (
    <SongCollectionItem
      itemKey={itemKey}
      fileId={fileId}
      onPlay={() => {
        if (!disabled) onPlay();
      }}
    >
      <div
        className={`relative min-w-0 rounded ${variant === "row" ? "flex items-center gap-3 border-b border-border/60 px-2 py-3" : "p-1"} ${current ? "bg-active text-primary" : ""}`}
      >
        <div className={variant === "row" ? "w-12 shrink-0" : "block w-full"}>
          <Artwork id={fileId} />
        </div>
        <div className={variant === "row" ? "min-w-0 flex-1" : "mt-3 min-w-0"}>
          <div
            className="block max-w-full truncate text-left font-medium"
            title={title}
          >
            {title}
          </div>
          {artist && (
            <Link
              to="/artists"
              search={{ artist }}
              className="block truncate text-xs text-muted-foreground w-fit hover:text-primary hover:underline"
            >
              {artist}
            </Link>
          )}
        </div>
        {favorite && <HeartButton fileId={fileId} />}
        {detail && (
          <div
            className={
              variant === "row"
                ? "shrink-0 text-xs tabular-nums text-muted-foreground"
                : "mt-1 truncate text-xs text-primary"
            }
          >
            {detail}
          </div>
        )}
      </div>
    </SongCollectionItem>
  );
}
