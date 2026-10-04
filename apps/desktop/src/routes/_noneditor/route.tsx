import { createFileRoute, Outlet } from "@tanstack/react-router";
import { useState, type CSSProperties } from "react";
import Sidebar from "../../components/Sidebar";
import NowPlaying from "../../components/NowPlaying";
import { TaskProgress } from "../../components/TaskProgress";

export const Route = createFileRoute("/_noneditor")({
  component: RouteComponent,
});

function RouteComponent() {
  const [sidebarWidth, setSidebarWidth] = useState(240);

  return (
    <div
      className="min-h-full w-full mt-14"
      style={{ "--sidebar-width": `${sidebarWidth}px` } as CSSProperties}
    >
      <Sidebar width={sidebarWidth} onWidthChange={setSidebarWidth} />
      <div
        className="min-h-full min-w-0 transition-[padding] duration-150"
        style={{
          paddingLeft: sidebarWidth,
          paddingRight: "var(--queue-width, 0px)",
        }}
      >
        <Outlet />

        <TaskProgress />

        <NowPlaying />
      </div>
    </div>
  );
}
