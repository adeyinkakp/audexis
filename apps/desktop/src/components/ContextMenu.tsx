import {
  cloneElement,
  isValidElement,
  useRef,
  useState,
  type HTMLAttributes,
  type MouseEvent,
  type ReactElement,
  type ReactNode,
} from "react";
import * as DropdownMenu from "@radix-ui/react-dropdown-menu";
import { Check, ChevronRight } from "lucide-react";

export type MenuOptions = Array<MenuItem | { item: string }>;
export type MenuItem = {
  text: string;
  action?: () => void | Promise<void>;
  disabled?: boolean;
  checked?: boolean;
  submenu?: MenuOptions | (() => MenuOptions);
};

const panelClass =
  "z-[10000] max-h-[var(--radix-dropdown-menu-content-available-height)] min-w-48 max-w-xs overflow-auto rounded-md border border-border bg-popover p-1 text-xs text-foreground shadow-lg";
const itemClass =
  "relative flex cursor-default select-none items-center gap-2 rounded px-2 py-1.5 outline-none data-[highlighted]:bg-muted data-[state=open]:bg-muted data-[disabled]:pointer-events-none data-[disabled]:opacity-40";

export function ContextMenuArea({
  items,
  children,
  asChild,
  className,
}: {
  items: () => MenuOptions;
  children: ReactNode;
  className?: string;
  asChild?: boolean;
}) {
  const [position, setPosition] = useState<{ x: number; y: number } | null>(
    null,
  );
  const previousFocus = useRef<HTMLElement | null>(null);
  const open = (event: MouseEvent<HTMLElement>) => {
    event.preventDefault();
    event.stopPropagation();
    previousFocus.current = document.activeElement as HTMLElement | null;
    setPosition({ x: event.clientX, y: event.clientY });
  };
  const content =
    asChild && isValidElement<HTMLAttributes<HTMLElement>>(children) ? (
      cloneElement(children as ReactElement<HTMLAttributes<HTMLElement>>, {
        className: [children.props.className, className]
          .filter(Boolean)
          .join(" "),
        onContextMenu: (event) => {
          children.props.onContextMenu?.(event);
          if (!event.defaultPrevented) open(event);
        },
      })
    ) : (
      <div className={className} onContextMenu={open}>
        {children}
      </div>
    );

  return (
    <DropdownMenu.Root
      modal={false}
      open={position !== null}
      onOpenChange={(isOpen) => {
        if (!isOpen) setPosition(null);
      }}
    >
      {content}
      <DropdownMenu.Trigger
        tabIndex={-1}
        aria-hidden="true"
        className="pointer-events-none fixed h-px w-px opacity-0"
        style={{ left: position?.x ?? 0, top: position?.y ?? 0 }}
      />
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          className={panelClass}
          side="bottom"
          align="start"
          sideOffset={0}
          collisionPadding={4}
          onContextMenu={(event) => {
            event.preventDefault();
            event.stopPropagation();
          }}
          onCloseAutoFocus={(event) => {
            event.preventDefault();
            if (previousFocus.current?.isConnected)
              previousFocus.current.focus({ preventScroll: true });
          }}
        >
          <MenuEntries items={items()} />
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}

function MenuEntries({ items }: { items: MenuOptions }) {
  return (
    <>
      {items.map((item, index) => {
        if ("item" in item)
          return (
            <DropdownMenu.Separator
              key={index}
              className="my-1 border-t border-border"
            />
          );
        if (item.submenu) {
          const children =
            typeof item.submenu === "function" ? item.submenu() : item.submenu;
          return (
            <DropdownMenu.Sub key={index}>
              <DropdownMenu.SubTrigger
                disabled={item.disabled || children.length === 0}
                className={itemClass}
              >
                <span className="min-w-0 flex-1 truncate">{item.text}</span>
                <ChevronRight size={13} aria-hidden="true" />
              </DropdownMenu.SubTrigger>
              <DropdownMenu.Portal>
                <DropdownMenu.SubContent
                  className={panelClass}
                  sideOffset={2}
                  collisionPadding={4}
                  onContextMenu={(event) => {
                    event.preventDefault();
                    event.stopPropagation();
                  }}
                >
                  <MenuEntries items={children} />
                </DropdownMenu.SubContent>
              </DropdownMenu.Portal>
            </DropdownMenu.Sub>
          );
        }
        const select = () => {
          void Promise.resolve()
            .then(() => item.action?.())
            .catch((error) =>
              console.error("Context menu action failed", error),
            );
        };
        if (item.checked !== undefined)
          return (
            <DropdownMenu.CheckboxItem
              key={index}
              checked={item.checked}
              disabled={item.disabled}
              onSelect={select}
              className={itemClass}
            >
              <span className="w-3">
                <DropdownMenu.ItemIndicator>
                  <Check size={12} />
                </DropdownMenu.ItemIndicator>
              </span>
              <span className="truncate">{item.text}</span>
            </DropdownMenu.CheckboxItem>
          );
        return (
          <DropdownMenu.Item
            key={index}
            disabled={item.disabled}
            onSelect={select}
            className={itemClass}
          >
            {item.text}
          </DropdownMenu.Item>
        );
      })}
    </>
  );
}
