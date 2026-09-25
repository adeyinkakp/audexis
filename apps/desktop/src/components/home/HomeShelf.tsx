import { useRef, type ReactNode } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
export function HomeShelf({
  title,
  subtitle,
  action,
  children,
}: {
  title: string;
  subtitle?: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const scroll = (direction: number) =>
    ref.current?.scrollBy({
      left: direction * ref.current.clientWidth * 0.8,
      behavior:
        window.matchMedia("(prefers-reduced-motion: reduce)").matches ||
        document.documentElement.dataset.reduceMotion === "true"
          ? "instant"
          : "smooth",
    });
  return (
    <section className="min-w-0 py-6">
      <header className="mb-5 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="text-2xl font-semibold">{title}</h2>
          {subtitle && (
            <p className="mt-1 text-sm text-muted-foreground">{subtitle}</p>
          )}
        </div>
        <div className="flex items-center gap-2">
          {action}
          <button
            aria-label={`Scroll ${title} left`}
            onClick={() => scroll(-1)}
            className="rounded-full p-2 hover:bg-muted"
          >
            <ChevronLeft size={19} />
          </button>
          <button
            aria-label={`Scroll ${title} right`}
            onClick={() => scroll(1)}
            className="rounded-full p-2 hover:bg-muted"
          >
            <ChevronRight size={19} />
          </button>
        </div>
      </header>
      <div
        ref={ref}
        tabIndex={0}
        aria-label={title}
        className="flex snap-x snap-proximity gap-5 overflow-x-auto overscroll-x-contain pb-4 pt-1 [&>*]:w-44 [&>*]:shrink-0 [&>*]:snap-start sm:[&>*]:w-48"
      >
        {children}
      </div>
    </section>
  );
}
