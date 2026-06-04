import type { ReactNode } from "react";

export function DataTable({
  children,
  empty,
  isEmpty,
}: {
  children: ReactNode;
  empty?: ReactNode;
  isEmpty?: boolean;
}) {
  if (isEmpty && empty) {
    return <div className="dw-table-empty-state">{empty}</div>;
  }
  return (
    <div className="overflow-x-auto">
      <table className="dw-table">{children}</table>
    </div>
  );
}

export function DataTableEmpty({
  icon,
  message,
}: {
  icon?: ReactNode;
  message: string;
}) {
  return (
    <div className="dw-table-empty-state">
      {icon}
      <p className="text-sm text-secondary m-0">{message}</p>
    </div>
  );
}
