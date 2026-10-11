import {
  forwardRef,
  createContext,
  useContext,
  useCallback,
  useMemo,
  type ButtonHTMLAttributes,
  useEffect,
  useId,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { cn } from "../utils";

const ModalContext = createContext<{
  close: () => void;
  register: (id: string, dirty: boolean, busy: boolean) => void;
  footer: HTMLDivElement | null;
} | null>(null);
const SectionContext = createContext(true);

export function ModalSection({
  active,
  children,
}: {
  active: boolean;
  children: ReactNode;
}) {
  return (
    <SectionContext.Provider value={active}>
      <div hidden={!active} className="min-h-full">
        {children}
      </div>
    </SectionContext.Provider>
  );
}

export function useModalChanges(hasChanges: boolean, busy = false) {
  const context = useContext(ModalContext);
  const register = context?.register;
  const id = useId();
  useEffect(() => {
    register?.(id, hasChanges, busy);
    return () => register?.(id, false, false);
  }, [register, id, hasChanges, busy]);
}

export function ModalActions({ children }: { children: ReactNode }) {
  const context = useContext(ModalContext);
  const active = useContext(SectionContext);
  if (!active) return null;
  const actions = (
    <div className="flex w-full flex-wrap items-center justify-end gap-2 px-6 py-4">
      {children}
    </div>
  );
  return context?.footer ? createPortal(actions, context.footer) : null;
}

export function ModalCloseButton(
  props: ButtonHTMLAttributes<HTMLButtonElement>,
) {
  const context = useContext(ModalContext);
  return <button {...props} type="button" onClick={context?.close} />;
}

const FOCUSABLE_SELECTOR =
  'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';

function getOrCreateModalRoot(id: string): HTMLElement {
  let root = document.getElementById(id);
  if (!root) {
    root = document.createElement("div");
    root.id = id;
    document.body.appendChild(root);
  }
  return root;
}

function getFocusable(container: HTMLElement | null): HTMLElement[] {
  if (!container) return [];
  return Array.from(
    container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR),
  ).filter((el) => {
    const isDisabled = (el as HTMLButtonElement).disabled;
    const isHidden = el.getAttribute("aria-hidden") === "true";
    const hasDisabledAttr = el.getAttribute("disabled") !== null;
    return (
      !isDisabled &&
      !isHidden &&
      !hasDisabledAttr &&
      el.getClientRects().length > 0
    );
  });
}

export interface ModalProps {
  open: boolean;
  onClose: () => void;
  hasChanges?: boolean;
  busy?: boolean;

  title?: ReactNode;
  description?: ReactNode;
  header?: ReactNode;
  footer?: ReactNode;
  sizeMax?: boolean;

  closeOnOverlayClick?: boolean;
  closeOnEsc?: boolean;
  trapFocus?: boolean;
  preventScroll?: boolean;

  initialFocusRef?: React.RefObject<HTMLElement | null>;

  zIndexClassName?: string;
  overlayClassName?: string;
  panelClassName?: string;
  bodyClassName?: string;
  footerClassName?: string;
  showCloseButton?: boolean;

  children: ReactNode;
}

export const Modal = forwardRef<HTMLDivElement, ModalProps>(function Modal(
  {
    open,
    onClose,
    hasChanges = false,
    busy = false,
    title,
    description,
    header,
    footer,

    closeOnOverlayClick = true,
    closeOnEsc = true,
    trapFocus = true,
    preventScroll = true,
    sizeMax = false,
    initialFocusRef,
    zIndexClassName = "z-[12000]",
    overlayClassName,
    panelClassName,
    bodyClassName,
    footerClassName,
    showCloseButton = true,
    children,
  },
  forwardedRef,
) {
  const internalRef = useRef<HTMLDivElement | null>(null);
  const [portalNode, setPortalNode] = useState<HTMLElement | null>(null);

  const [changes, setChanges] = useState<
    Record<string, { dirty: boolean; busy: boolean }>
  >({});
  const [closeError, setCloseError] = useState(false);
  const [footerNode, setFooterNode] = useState<HTMLDivElement | null>(null);
  const register = useCallback(
    (id: string, dirty: boolean, pending: boolean) => {
      setChanges((current) => {
        if (!dirty && !pending) {
          if (!(id in current)) return current;
          const next = { ...current };
          delete next[id];
          return next;
        }
        return { ...current, [id]: { dirty, busy: pending } };
      });
    },
    [],
  );
  const dirty =
    hasChanges || Object.values(changes).some((value) => value.dirty);
  const pending = busy || Object.values(changes).some((value) => value.busy);
  const requestClose = useCallback(() => {
    if (dirty || pending) {
      setCloseError(true);
      return;
    }
    onClose();
  }, [dirty, pending, onClose]);
  useEffect(() => {
    if (!open || (!dirty && !pending)) setCloseError(false);
  }, [open, dirty, pending]);
  const context = useMemo(
    () => ({ close: requestClose, register, footer: footerNode }),
    [requestClose, register, footerNode],
  );

  const titleId = useId();
  const descId = useId();

  const mergedRef = (node: HTMLDivElement | null) => {
    internalRef.current = node;
    if (!forwardedRef) return;
    if (typeof forwardedRef === "function") forwardedRef(node);
    else forwardedRef.current = node;
  };

  useEffect(() => {
    if (!open) return;
    setPortalNode(getOrCreateModalRoot("modal-root"));
  }, [open]);

  useEffect(() => {
    if (!open || !preventScroll) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = prev;
    };
  }, [open, preventScroll]);

  useEffect(() => {
    if (!open) return;

    const previousFocus = document.activeElement as HTMLElement | null;
    const focus = () => {
      const container = internalRef.current;
      if (!container) return;
      const target = initialFocusRef?.current;
      if (target) {
        target.focus();
        return;
      }
      const focusable = getFocusable(container);
      focusable[0]?.focus();
    };

    const id = window.setTimeout(focus, 30);
    return () => {
      window.clearTimeout(id);
      if (previousFocus?.isConnected)
        previousFocus.focus({ preventScroll: true });
    };
  }, [open, initialFocusRef]);

  useEffect(() => {
    if (!open) return;

    const onKeyDown = (e: KeyboardEvent) => {
      const panels = document.querySelectorAll("[data-audexis-modal]");
      if (panels[panels.length - 1] !== internalRef.current) return;
      if (e.key === "Escape" && closeOnEsc) {
        e.preventDefault();
        requestClose();
        return;
      }

      if (!trapFocus || e.key !== "Tab") return;

      const container = internalRef.current;
      const focusable = getFocusable(container);
      if (focusable.length === 0) return;

      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const active = document.activeElement;

      if (e.shiftKey) {
        if (active === first) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (active === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };

    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [open, closeOnEsc, requestClose, trapFocus]);

  if (!open || !portalNode) return null;

  return createPortal(
    <ModalContext.Provider value={context}>
      <div
        aria-modal="true"
        role="dialog"
        aria-labelledby={title ? titleId : undefined}
        aria-describedby={description ? descId : undefined}
        className={cn(
          "fixed inset-x-0 bottom-0 top-14 flex items-center justify-center p-4 md:p-8",
          zIndexClassName,
        )}
      >
        <div
          className={cn(
            "absolute inset-0 bg-background/70 backdrop-blur-sm border-t border-border animate-in fade-in",
            overlayClassName,
          )}
          onClick={closeOnOverlayClick ? requestClose : undefined}
        />

        <div
          ref={mergedRef}
          data-audexis-modal
          className={cn(
            "relative w-full max-w-4xl rounded-lg border border-border bg-linear-to-b from-background/95 to-background/80 shadow-xl ring-1 ring-border/50 overflow-hidden",
            "flex flex-col max-h-full",
            sizeMax ? "h-[85vh] w-full" : "",
            "animate-in duration-150",

            panelClassName,
          )}
          onClick={(e) => e.stopPropagation()}
        >
          {header !== undefined && title && (
            <h2 id={titleId} className="sr-only">
              {title}
            </h2>
          )}
          {header !== undefined && description && (
            <div id={descId} className="sr-only">
              {description}
            </div>
          )}
          {header !== undefined ? (
            header
          ) : (
            <div className="shrink-0 flex items-center gap-4 py-2 px-6 h-14 border-b border-border/60 bg-background/70 backdrop-blur-sm">
              <div className="min-w-0">
                {title && (
                  <h2
                    id={titleId}
                    className="text-sm font-semibold tracking-wide uppercase text-foreground/70"
                  >
                    {title}
                  </h2>
                )}
                {description && (
                  <div
                    id={descId}
                    className="text-[11px] text-foreground/50 font-medium"
                  >
                    {description}
                  </div>
                )}
              </div>
              {showCloseButton && (
                <div className="ml-auto flex items-center gap-2">
                  <button
                    onClick={requestClose}
                    className="group h-8 w-8 rounded-md border border-border/60 bg-background/40 hover:bg-destructive/10 hover:border-destructive/50 transition-all duration-200 flex items-center justify-center"
                    aria-label="Close"
                  >
                    <span className="text-foreground/60 group-hover:text-destructive text-lg leading-none transition-colors">
                      ×
                    </span>
                  </button>
                </div>
              )}
            </div>
          )}

          <div
            className={cn(
              "min-h-0 flex-1 overflow-auto custom-scrollbar",
              bodyClassName,
            )}
          >
            {children}
          </div>

          {closeError && (
            <p
              role="alert"
              className="shrink-0 border-t border-border bg-background px-6 py-3 text-sm text-destructive"
            >
              {pending
                ? "Please wait for saving to finish."
                : "You have unsaved changes. Save or discard them before closing. Check other tabs if needed."}
            </p>
          )}
          <div
            ref={setFooterNode}
            className="shrink-0 border-border bg-background [&:not(:empty)]:border-t"
          />
          {footer && (
            <div
              className={cn(
                "shrink-0 flex items-center justify-end gap-2 px-6 min-h-14 border-t border-border/60 bg-background/70 backdrop-blur-sm",
                footerClassName,
              )}
            >
              {footer}
            </div>
          )}
        </div>
      </div>
    </ModalContext.Provider>,
    portalNode,
  );
});
