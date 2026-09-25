import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowUpRight, Disc3, Heart, ListMusic, Search } from "lucide-react";
import { HomeDiscovery } from "../../components/home/HomeDiscovery";
export const Route = createFileRoute("/_noneditor/")({ component: HomePage });
function HomePage() {
  const hour = new Date().getHours();
  const greeting =
    hour < 12
      ? "Good morning."
      : hour < 18
        ? "Good afternoon."
        : "Good evening.";
  return (
    <main className="h-[calc(100dvh-5.5rem)] overflow-auto px-6 pb-10 pt-7 lg:px-9">
      <header className="mb-7 flex items-end justify-between gap-4">
        <div className="flex flex-col gap-4 w-full">
          <h1 className="text-3xl font-bold tracking-tight lg:text-4xl">
            {greeting}
          </h1>
          <Link
            to="/search"
            className="flex w-full items-center gap-2 rounded-full border border-border bg-muted/20 px-4 py-2.5 text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          >
            <Search size={17} />
            <span className="hidden sm:inline">Search your library</span>
          </Link>
        </div>
      </header>

      <HomeDiscovery />
    </main>
  );
}
