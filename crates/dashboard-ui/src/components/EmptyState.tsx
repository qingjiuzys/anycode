import { Icon } from "@/components/Icon";

export function EmptyState({
  title,
  description,
  icon = "inbox",
}: {
  title: string;
  description?: string;
  icon?: string;
}) {
  return (
    <div className="dw-empty">
      <Icon name={icon} size={40} className="text-outline mb-3" />
      <h3 className="text-base font-semibold text-on-surface m-0">{title}</h3>
      {description && <p className="text-sm text-secondary mt-2 max-w-md">{description}</p>}
    </div>
  );
}
