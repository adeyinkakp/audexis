import { useEffect, useRef } from "react";

export function InfiniteScroll({
  hasMore,
  fetching,
  error,
  enabled = true,
  itemCount,
  loadMore,
}: {
  hasMore: boolean;
  fetching: boolean;
  error: boolean;
  enabled?: boolean;
  itemCount: number;
  loadMore: () => Promise<unknown>;
}) {
  const sentinel = useRef<HTMLDivElement>(null);
  const loading = useRef(false);
  useEffect(() => {
    const element = sentinel.current;
    if (!element || !enabled || !hasMore || fetching || error) return;
    let root = element.parentElement;
    while (root && !/(auto|scroll)/.test(getComputedStyle(root).overflowY)) {
      root = root.parentElement;
    }
    let active = true;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (!active || !entry.isIntersecting || loading.current) return;
        loading.current = true;
        void loadMore()
          .catch(() => {})
          .finally(() => {
            loading.current = false;
          });
      },
      { root, rootMargin: "0px 0px 240px 0px" },
    );
    observer.observe(element);
    return () => {
      active = false;
      observer.disconnect();
    };
  }, [enabled, hasMore, fetching, error, itemCount, loadMore]);

  if (!enabled || !hasMore) return null;
  return (
    <div
      ref={sentinel}
      className="flex min-h-12 items-center justify-center py-3"
    >
      {error ? (
        <button
          type="button"
          disabled={fetching}
          onClick={() => void loadMore().catch(() => {})}
          className="rounded-lg border border-border px-4 py-2 text-sm disabled:opacity-50"
        >
          Could not load more. Retry
        </button>
      ) : fetching ? (
        <p role="status" className="text-sm text-muted-foreground">
          Loading more…
        </p>
      ) : null}
    </div>
  );
}
