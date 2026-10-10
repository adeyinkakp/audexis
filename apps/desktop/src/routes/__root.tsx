import * as React from "react";
import { Outlet, createRootRoute } from "@tanstack/react-router";
import Titlebar from "../components/Titlebar";

export const Route = createRootRoute({
  component: RootComponent,
});

function RootComponent() {
  return (
    <React.Fragment>
      <Titlebar />
      <Outlet />
    </React.Fragment>
  );
}
