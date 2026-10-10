import { Heart } from "lucide-react";
import { useIsMutating } from "@tanstack/react-query";
import { useFavorites } from "../hooks/useFavorites";
export function HeartButton({ fileId }: { fileId: number }) {
  const favorites = useFavorites();
  const pending =
    useIsMutating({
      mutationKey: ["setMediaLoved"],
      predicate: (mutation) =>
        (mutation.state.variables as { fileId?: number } | undefined)
          ?.fileId === fileId,
    }) > 0;
  const loved = favorites.data?.includes(fileId) ?? false;
  const label = loved ? "Remove from Favorites" : "Add to Favorites";
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      aria-pressed={loved}
      disabled={pending || !favorites.data}
      onClick={(event) => {
        event.stopPropagation();
        favorites.setLoved({ fileId, loved: !loved });
      }}
      className={`shrink-0 rounded-full p-2 transition-colors hover:bg-primary/10 disabled:opacity-40 ${loved ? "text-primary" : "text-muted-foreground"}`}
    >
      <Heart size={17} fill={loved ? "currentColor" : "none"} />
    </button>
  );
}
