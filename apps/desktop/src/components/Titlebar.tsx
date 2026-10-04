export default function Titlebar() {
  return (
    <div
      data-tauri-drag-region={true}
      className="z-99 bg-popover/80  border-b border-border fixed h-14 top-0 w-full"
    ></div>
  );
}
