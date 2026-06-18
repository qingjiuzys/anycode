import type { ReactNode } from "react";
import { Link } from "@tanstack/react-router";
import { useControlCenter } from "@/context/ControlCenterContext";
import { useEmbeddedControlCenter } from "@/context/EmbeddedControlCenterContext";
import { buildControlCenterHref } from "@/lib/controlCenterPaths";

type Props = {
  to: string;
  params?: Record<string, string>;
  search?: Record<string, string | undefined>;
  className?: string;
  children: ReactNode;
  title?: string;
  onClick?: () => void;
};

export function ControlCenterLink({
  to,
  params,
  search,
  className = "",
  children,
  title,
  onClick,
}: Props) {
  const embedded = useEmbeddedControlCenter();
  const { setActivePath } = useControlCenter();

  if (embedded) {
    const href = buildControlCenterHref(to, params, search);
    return (
      <button
        type="button"
        title={title}
        className={`${className} border-0 bg-transparent p-0 cursor-pointer font-inherit text-inherit`}
        onClick={() => {
          onClick?.();
          setActivePath(href);
        }}
      >
        {children}
      </button>
    );
  }

  return (
    <Link
      to={to}
      params={params}
      search={search}
      className={className}
      title={title}
      onClick={onClick}
    >
      {children}
    </Link>
  );
}
