import type { ReactNode } from "react";

export function FilterBar({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={`flex flex-wrap items-center gap-2 mb-4 ${className ?? ""}`}>
      {children}
    </div>
  );
}
