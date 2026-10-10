export function createSongSelectionMenu() {
  const collections = new Map<symbol, () => void>();
  let active: symbol | undefined;
  return {
    register(selectAll: () => void) {
      const id = Symbol();
      collections.set(id, selectAll);
      return {
        activate: () => {
          active = id;
        },
        unregister: () => {
          collections.delete(id);
          if (active === id) active = undefined;
        },
      };
    },
    selectAll() {
      const select =
        (active && collections.get(active)) ||
        collections.values().next().value;
      select?.();
    },
  };
}

export const songSelectionMenu = createSongSelectionMenu();
