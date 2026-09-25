import { useEffect, useRef, useState } from "react";
import { useStore } from "../hooks/useStore";
import { cn } from "../utils";

export default function MarqueeText({
  children,
  className,
}: {
  children: string;
  className?: string;
}) {
  const { preferences } = useStore();
  const viewportRef = useRef<HTMLDivElement>(null);
  const stripRef = useRef<HTMLDivElement>(null);
  const textRef = useRef<HTMLSpanElement>(null);
  const [scrolling, setScrolling] = useState(false);

  useEffect(() => {
    const viewport = viewportRef.current;
    const strip = stripRef.current;
    const text = textRef.current;
    if (!viewport || !strip || !text) return;

    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    let animation: Animation | undefined;
    const measure = () => {
      animation?.cancel();
      const overflow =
        text.getBoundingClientRect().width > viewport.clientWidth + 1;
      const shouldScroll =
        overflow && !reducedMotion.matches && !preferences.reduceMotion;
      setScrolling(shouldScroll);
      if (!shouldScroll) return;

      const distance = text.getBoundingClientRect().width + 32;
      const travelTime = (distance / 30) * 1000;
      const duration = travelTime + 2000;
      animation = strip.animate(
        [
          { transform: "translateX(0)", offset: 0 },
          { transform: "translateX(0)", offset: 2000 / duration },
          { transform: `translateX(-${distance}px)`, offset: 1 },
        ],
        { duration, iterations: Infinity, easing: "linear" },
      );
    };

    const observer = new ResizeObserver(measure);
    observer.observe(viewport);
    observer.observe(text);
    reducedMotion.addEventListener("change", measure);
    measure();
    return () => {
      observer.disconnect();
      reducedMotion.removeEventListener("change", measure);
      animation?.cancel();
    };
  }, [children, preferences.reduceMotion]);

  return (
    <div
      ref={viewportRef}
      title={children}
      className={cn("overflow-hidden", className)}
    >
      <div
        ref={stripRef}
        className="flex w-max items-center gap-8 whitespace-nowrap"
      >
        <span ref={textRef}>{children}</span>
        {scrolling && <span aria-hidden="true">{children}</span>}
      </div>
    </div>
  );
}
