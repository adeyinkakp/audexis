import { useEffect, useRef, useState } from "react";
import { useStore } from "./useStore";
import {
  defaultColumnOrder,
  MAX_COLUMN_WIDTH,
  MIN_COLUMN_WIDTH,
  songColumns,
  type SongColumnId,
} from "../components/songs/columns";
import toast from "react-hot-toast";

type Preferences = {
  order: SongColumnId[];
  widths: Partial<Record<SongColumnId, number>>;
};
const KEY = "songs-table-columns-v1";
let writes = Promise.resolve();
const defaults = (): Preferences => ({
  order: [...defaultColumnOrder],
  widths: {},
});
function normalize(value: unknown): Preferences {
  if (!value || typeof value !== "object") return defaults();
  const raw = value as Partial<Preferences>;
  const valid = new Set<string>(songColumns.map((column) => column.id));
  const order = Array.isArray(raw.order)
    ? [...new Set(raw.order.filter((id) => valid.has(id)))]
    : [...defaultColumnOrder];
  const widths: Preferences["widths"] = {};
  for (const column of songColumns) {
    const width = raw.widths?.[column.id];
    if (typeof width === "number" && Number.isFinite(width))
      widths[column.id] = Math.max(
        MIN_COLUMN_WIDTH,
        Math.min(MAX_COLUMN_WIDTH, width),
      );
  }
  return { order, widths };
}

export function useSongColumns() {
  const { store } = useStore();
  const [preferences, setPreferences] = useState(defaults);
  const [ready, setReady] = useState(false);
  const current = useRef(preferences);
  useEffect(() => {
    let disposed = false;
    void writes
      .then(() => store.get<unknown>(KEY))
      .then((saved) => {
        if (disposed) return;
        current.current = normalize(saved);
        setPreferences(current.current);
        setReady(true);
      })
      .catch(() => {
        if (!disposed) {
          setReady(true);
          toast.error("Could not load column settings");
        }
      });
    return () => {
      disposed = true;
    };
  }, [store]);

  const persist = () => {
    const snapshot = current.current;
    writes = writes
      .then(async () => {
        await store.set(KEY, snapshot);
        await store.save();
      })
      .catch(() => {
        toast.error("Could not save column settings");
      });
  };
  const update = (next: Preferences, save = true) => {
    if (!ready) return;
    current.current = next;
    setPreferences(next);
    if (save) persist();
  };
  return {
    ...preferences,
    ready,
    persist,
    toggle: (id: SongColumnId) =>
      update({
        ...current.current,
        order: current.current.order.includes(id)
          ? current.current.order.filter((item) => item !== id)
          : [id, ...current.current.order],
      }),
    reorder: (order: SongColumnId[]) => update({ ...current.current, order }),
    resize: (id: SongColumnId, width: number, save = false) =>
      update(
        {
          ...current.current,
          widths: {
            ...current.current.widths,
            [id]: Math.max(MIN_COLUMN_WIDTH, Math.min(MAX_COLUMN_WIDTH, width)),
          },
        },
        save,
      ),
  };
}
