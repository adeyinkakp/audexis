export type SongSelectionItem = { key: string; fileId: number };

export function selectedItems(
  items: SongSelectionItem[],
  keys: ReadonlySet<string>,
) {
  return items.filter((item) => item.fileId > 0 && keys.has(item.key));
}

export function toggleSongSelection(
  items: SongSelectionItem[],
  selected: ReadonlySet<string>,
  key: string,
  anchor: string | null,
  range: boolean,
): Set<string> {
  const available = items.filter((item) => item.fileId > 0);
  const next = new Set(
    selectedItems(available, selected).map((item) => item.key),
  );
  const end = available.findIndex((item) => item.key === key);
  if (end < 0) return next;
  const start = available.findIndex((item) => item.key === anchor);
  if (range && start >= 0) {
    for (const item of available.slice(
      Math.min(start, end),
      Math.max(start, end) + 1,
    ))
      next.add(item.key);
  } else if (next.has(key)) next.delete(key);
  else next.add(key);
  return next;
}

export function nextInsertionOrder(
  ids: number[],
  queueIsEmpty: boolean,
): number[] {
  return queueIsEmpty
    ? [ids[0], ...ids.slice(1).reverse()].filter((id) => id !== undefined)
    : [...ids].reverse();
}

export function selectSongSelection(
  items: SongSelectionItem[],
  selected: ReadonlySet<string>,
  key: string,
  anchor: string | null,
  range = false,
  additive = false,
): Set<string> {
  if (!items.some((item) => item.key === key && item.fileId > 0))
    return new Set(selected);
  if (!range && !additive) return new Set([key]);
  return toggleSongSelection(items, selected, key, anchor, range);
}
