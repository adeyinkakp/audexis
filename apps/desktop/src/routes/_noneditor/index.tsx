import { createFileRoute, Link } from "@tanstack/react-router";
import { Search } from "lucide-react";
import { HomeDiscovery } from "../../components/home/HomeDiscovery";
import { useHomeDiscovery } from "../../hooks/useHomeDiscovery";
export const Route = createFileRoute("/_noneditor/")({ component: HomePage });

function HomePage() {
  const discovery = useHomeDiscovery("all");
  const hour = new Date().getHours();
  const greeting =
    hour < 12
      ? "Good morning."
      : hour < 18
        ? "Good afternoon."
        : "Good evening.";
  return (
    <main className="h-[calc(100dvh-3rem)] overflow-auto px-6 pb-10 pt-7 lg:px-9">
      <header className="mb-7 flex items-end justify-between gap-4">
        <div className="flex flex-col gap-4 w-full">
          <Link
            to="/search"
            className="flex w-full items-center gap-2 rounded-full border border-border bg-muted/20 px-4 py-2.5 text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          >
            <Search size={17} />
            <span className="hidden sm:inline">Search your library</span>
          </Link>
          <h1 className="text-3xl font-bold tracking-tight lg:text-4xl">
            {greeting}
          </h1>
          {discovery.data && (
            <dl className="grid grid-cols-2 gap-3 lg:grid-cols-4">
              {[
                [
                  "Tracks",
                  discovery.data.summary.total_tracks.toLocaleString(),
                ],
                [
                  "Library size",
                  formatBytes(discovery.data.summary.total_size),
                ],
                ["Albums", discovery.data.summary.album_count.toLocaleString()],
                [
                  "Last scanned",
                  getRelativeTime(
                    (discovery.data.summary.last_scanned ?? 0) * 1000,
                  ),
                ],
              ].map(([label, value]) => (
                <div
                  key={label}
                  className="min-w-0 rounded-2xl border border-border/60 bg-card/40 px-4 py-3"
                >
                  <dt className="text-xs text-muted-foreground">{label}</dt>
                  <dd
                    className="mt-1 truncate text-sm font-semibold"
                    title={value}
                  >
                    {value}
                  </dd>
                </div>
              ))}
            </dl>
          )}
        </div>
      </header>

      <HomeDiscovery selection={discovery} />
    </main>
  );
}

function getRelativeTime(time: number) {
  const rtf = new Intl.RelativeTimeFormat("en", { numeric: "auto" });
  const dateObj = new Date(time);
  console.log({ dateObj });

  const timeDiff = dateObj.getTime() - Date.now();

  const seconds = timeDiff / 1000;

  const DIVISIONS = [
    { amount: 60, name: "seconds" },
    { amount: 60, name: "minutes" },
    { amount: 24, name: "hours" },
    { amount: 7, name: "days" },
    { amount: 4.345, name: "weeks" },
    { amount: 12, name: "months" },
    { amount: Number.POSITIVE_INFINITY, name: "years" },
  ];

  let duration = seconds;
  for (const division of DIVISIONS) {
    if (Math.abs(duration) < division.amount) {
      // dk how to get RelativeTimeFormatUnit type
      return rtf.format(Math.round(duration), division.name as "minutes");
    }
    duration /= division.amount;
  }
}

function formatBytes(bytes: number) {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const unit = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  return `${(bytes / 1024 ** unit).toFixed(unit > 1 ? 1 : 0)} ${units[unit]}`;
}
