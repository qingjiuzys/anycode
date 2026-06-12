import type { ReactNode, MouseEvent, PointerEvent } from "react";
import { openExternal } from "@/lib/openExternal";

export function ExternalNavLink({
  href,
  className,
  children,
}: {
  href: string;
  className?: string;
  children: ReactNode;
}) {
  const open = () => {
    void openExternal(href);
  };

  const stop = (e: MouseEvent | PointerEvent) => {
    e.stopPropagation();
  };

  const onClick = (e: MouseEvent<HTMLAnchorElement>) => {
    stop(e);
    e.preventDefault();
    open();
  };

  return (
    <a
      href={href}
      className={className}
      onPointerDown={stop}
      onClick={onClick}
      rel="noopener noreferrer"
    >
      {children}
    </a>
  );
}
