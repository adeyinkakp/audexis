import { useContext } from "react";
import { StoreContext } from "./StoreContext";

export type { ThemePreference, RowDensity } from "./StoreContext";

export function useStore() {
  const context = useContext(StoreContext);
  if (!context) throw new Error("useStore must be used within StoreProvider");
  return context;
}
