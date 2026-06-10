import { useMemo, useState } from "react";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

type Props = {
  title: string;
  subtitle?: string;
  defaultOpen?: boolean;
  tone?: "default" | "error" | "running" | "muted";
  icon?: string;
  headerActions?: React.ReactNode;
  children: React.ReactNode;
};

export function CollapsiblePanel({
  title,
  subtitle,
  defaultOpen = false,
  tone = "default",
  icon = "chevron_right",
  headerActions,
  children,
}: Props) {
  const t = useT();
  const [open, setOpen] = useState(defaultOpen);

  const toneClass =
    tone === "error"
      ? "border-error/25 bg-error-container/10"
      : tone === "running"
        ? "border-primary/25 bg-primary-container/10"
        : tone === "muted"
          ? "border-outline-variant/60 bg-surface-container-lowest"
          : "border-outline-variant/80 bg-surface-container-lowest";

  return (
    <div className={`rounded-xl border overflow-hidden ${toneClass}`}>
      <button
        type="button"
        className="w-full flex items-center gap-2 px-3 py-2.5 text-left bg-transparent border-0 cursor-pointer hover:bg-surface-container-low/80 transition-colors"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
      >
        <Icon
          name={open ? "expand_more" : "chevron_right"}
          size={18}
          className="text-secondary shrink-0"
        />
        {icon && icon !== "chevron_right" && (
          <Icon name={icon} size={16} className="text-secondary shrink-0" />
        )}
        <span className="flex-1 min-w-0">
          <span className="block text-sm font-medium truncate text-on-surface">{title}</span>
          {subtitle && (
            <span className="block text-xs text-secondary truncate mt-0.5">{subtitle}</span>
          )}
        </span>
        {headerActions && (
          <span
            className="shrink-0 flex items-center gap-1"
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => e.stopPropagation()}
          >
            {headerActions}
          </span>
        )}
        <span className="sr-only">{open ? t("common.collapse") : t("common.expand")}</span>
      </button>
      {open && <div className="px-3 pb-3 pt-0 border-t border-outline-variant/50">{children}</div>}
    </div>
  );
}

export function previewLines(text: string, maxLines = 2, maxChars = 160): string {
  const lines = text.split("\n").map((l) => l.trim()).filter(Boolean);
  if (lines.length === 0) return "";
  const head = lines.slice(0, maxLines).join(" · ");
  if (head.length <= maxChars) return head;
  return `${head.slice(0, maxChars)}…`;
}

export function contentStats(text: string): { lines: number; chars: number } {
  const trimmed = text.trim();
  return {
    lines: trimmed ? trimmed.split("\n").length : 0,
    chars: trimmed.length,
  };
}

export function useContentCollapse(text: string, force?: boolean) {
  return useMemo(() => {
    const { lines, chars } = contentStats(text);
    const long = lines > 8 || chars > 420;
    return {
      shouldCollapse: force ?? long,
      lines,
      chars,
    };
  }, [text, force]);
}
