import {
  CheckMenuItem,
  Menu,
  MenuItem,
  PredefinedMenuItem,
  Submenu,
} from "@tauri-apps/api/menu";
import type { MenuOptions } from "../components/ContextMenu";
import { dispatchMenuAction } from "./appMenu";

export type MenuGroup = {
  text: string;
  items: MenuOptions;
  disabled?: boolean;
};
type NativeItem = MenuItem | CheckMenuItem | PredefinedMenuItem | Submenu;
let windowMenu: Promise<Submenu> | undefined;

let latest: MenuGroup[] = [];
let signature = "";
let resources: (NativeItem | Menu)[] = [];
let entries = new Map<string, MenuItem | CheckMenuItem | Submenu>();
let previous = new Map<string, string>();
let pending = Promise.resolve();
function children(items: MenuOptions): MenuOptions {
  return items.map((item) =>
    "item" in item || !item.submenu
      ? item
      : {
          ...item,
          submenu: children(
            typeof item.submenu === "function" ? item.submenu() : item.submenu,
          ),
        },
  );
}
function lookup(path: string) {
  const [group, ...indices] = path.split(".").map(Number);
  let items = latest[group].items;
  for (const index of indices.slice(0, -1)) {
    const item = items[index];
    if ("item" in item || !Array.isArray(item.submenu)) return;
    items = item.submenu;
  }
  const item = items[indices[indices.length - 1]];
  return item && !("item" in item) ? item : undefined;
}
export function updateNativeMenus(groups: MenuGroup[]) {
  latest = groups.map((group) => ({ ...group, items: children(group.items) }));
  const work = pending
    .catch(() => undefined)
    .then(async () => {
      const snapshot = latest;
      const shape = JSON.stringify(snapshot, (key, value) =>
        ["action", "text", "disabled", "checked"].includes(key)
          ? key === "checked"
            ? typeof value === "boolean"
            : undefined
          : value,
      );
      if (shape !== signature) {
        const created: (NativeItem | Menu)[] = [];
        const nextEntries = new Map<
          string,
          MenuItem | CheckMenuItem | Submenu
        >();
        const track = <T extends NativeItem | Menu>(item: T): T => {
          created.push(item);
          return item;
        };
        const build = async (
          items: MenuOptions,
          prefix: string,
        ): Promise<NativeItem[]> => {
          const result: NativeItem[] = [];
          for (const [index, item] of items.entries()) {
            const path = `${prefix}.${index}`;
            if ("item" in item) {
              result.push(
                track(await PredefinedMenuItem.new({ item: "Separator" })),
              );
              continue;
            }
            const options = {
              text: item.text,
              enabled: !item.disabled,
              action: () => {
                const current = lookup(path);
                if (current && !current.disabled)
                  void Promise.resolve(current.action?.()).catch(console.error);
              },
            };
            const native = Array.isArray(item.submenu)
              ? await Submenu.new({
                  ...options,
                  items: await build(item.submenu, path),
                })
              : item.checked !== undefined
                ? await CheckMenuItem.new({ ...options, checked: item.checked })
                : await MenuItem.new(options);
            nextEntries.set(path, native);
            result.push(track(native));
          }
          return result;
        };
        try {
          const app = track(
            await Submenu.new({
              text: "Audexis",
              items: [
                { item: { About: { name: "Audexis" } } },
                { item: "Separator" },
                {
                  text: "Settings…",
                  accelerator: "CmdOrCtrl+,",
                  action: () => dispatchMenuAction("settings"),
                },
                { item: "Separator" },
                { item: "Services" },
                { item: "Separator" },
                { item: "Hide" },
                { item: "HideOthers" },
                { item: "ShowAll" },
                { item: "Separator" },
                { item: "Quit" },
              ],
            }),
          );
          const menus: Submenu[] = [app];
          for (const [index, group] of snapshot.entries()) {
            if (group.text === "Help") {
              windowMenu ??= Submenu.new({
                text: "Window",
                items: [
                  { item: "Minimize" },
                  { item: "Maximize" },
                  { item: "CloseWindow" },
                ],
              });
              menus.push(await windowMenu);
            }
            const submenu = track(
              await Submenu.new({
                text: group.text,
                enabled: !group.disabled,
                items: await build(group.items, String(index)),
              }),
            );
            nextEntries.set(String(index), submenu);
            menus.push(submenu);
          }
          const menu = track(await Menu.new({ items: menus }));
          await menu.setAsAppMenu();
          const old = resources;
          resources = created;
          entries = nextEntries;
          previous = new Map();
          signature = shape;
          await Promise.all(
            old.map((resource) => resource.close().catch(() => undefined)),
          );
        } catch (error) {
          await Promise.all(
            created.map((resource) => resource.close().catch(() => undefined)),
          );
          throw error;
        }
      }
      for (const [path, native] of entries) {
        const item = path.includes(".") ? lookup(path) : latest[Number(path)];
        if (!item) continue;
        const checked = "checked" in item ? item.checked : undefined;
        const state = JSON.stringify([item.text, item.disabled, checked]);
        if (previous.get(path) === state) continue;
        await native.setText(item.text);
        await native.setEnabled(!item.disabled);
        if (native instanceof CheckMenuItem && checked !== undefined)
          await native.setChecked(checked);
        previous.set(path, state);
      }
    });
  pending = work;
  return work;
}
