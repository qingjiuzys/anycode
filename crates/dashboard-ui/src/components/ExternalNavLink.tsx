import type { ReactNode, MouseEvent } from "react";
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
  const onClick = (e: MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault();
    void openExternal(href);
  };

  return (
    <a
      href={href}
      className={className}
      onClick={onClick}
      rel="noopener noreferrer"
      target="_blank"
    >
      {children}
    </a>
  );
}
