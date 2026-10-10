import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { motion, useReducedMotion } from "motion/react";
import { useStore } from "../hooks/useStore";
import MiniPlayer from "./MiniPlayer";

export default function ExpandedPlayer({ onClose }: { onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const systemReducedMotion = useReducedMotion();
  const { preferences } = useStore();
  const reduceMotion = preferences.reduceMotion || !!systemReducedMotion;

  useEffect(() => {
    const element = dialog.current;
    const previousFocus = document.activeElement as HTMLElement | null;
    const previousOverflow = document.body.style.overflow;
    element?.showModal();
    document.body.style.overflow = "hidden";
    return () => {
      element?.close();
      document.body.style.overflow = previousOverflow;
      previousFocus?.focus();
    };
  }, []);

  return createPortal(
    <motion.dialog
      ref={dialog}
      aria-label="Expanded player"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
      initial={{ y: reduceMotion ? 0 : "100%", opacity: 1 }}
      animate={{ y: 0, opacity: 1 }}
      exit={{ y: reduceMotion ? 0 : "100%", opacity: 1 }}
      transition={{
        duration: reduceMotion ? 0 : 0.35,
        ease: [0.22, 1, 0.36, 1],
      }}
      className="fixed inset-x-0 bottom-0 m-0 h-[calc(100%)] max-h-none w-full max-w-none overflow-hidden border-0 bg-background p-0 text-foreground backdrop:bg-transparent"
    >
      <MiniPlayer expanded onCollapse={onClose} reducedMotion={reduceMotion} />
    </motion.dialog>,
    document.body,
  );
}
