import { useRef, useState, type ReactNode } from "react";
import { Icon } from "@/components/Icon";

/**
 * Inline rename affordance: renders `children` with a pencil button next to it;
 * clicking the pencil swaps in an input (Enter/blur saves, Esc cancels).
 */
export function InlineRename({
  value,
  label,
  onSave,
  disabled,
  children,
  inputClassName,
  buttonClassName,
}: {
  value: string;
  label: string;
  onSave: (name: string) => void;
  disabled?: boolean;
  children: ReactNode;
  inputClassName?: string;
  buttonClassName?: string;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const cancelledRef = useRef(false);

  function startEditing() {
    setDraft(value);
    cancelledRef.current = false;
    setEditing(true);
  }

  function handleBlur() {
    setEditing(false);
    if (cancelledRef.current) {
      cancelledRef.current = false;
      return;
    }
    const trimmed = draft.trim();
    if (trimmed && trimmed !== value.trim()) {
      onSave(trimmed);
    }
  }

  if (editing) {
    return (
      <input
        // eslint-disable-next-line jsx-a11y/no-autofocus
        autoFocus
        className={inputClassName ?? "dw-input text-sm"}
        value={draft}
        maxLength={120}
        aria-label={label}
        onChange={(e) => setDraft(e.target.value)}
        onFocus={(e) => e.target.select()}
        onBlur={handleBlur}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.currentTarget.blur();
          } else if (e.key === "Escape") {
            cancelledRef.current = true;
            e.currentTarget.blur();
          }
        }}
      />
    );
  }

  return (
    <span className="inline-flex items-center gap-1 min-w-0">
      {children}
      <button
        type="button"
        title={label}
        aria-label={label}
        disabled={disabled}
        className={
          buttonClassName ??
          "dw-btn-ghost p-1 text-outline hover:text-on-surface opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity"
        }
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          startEditing();
        }}
      >
        <Icon name="edit" size={14} />
      </button>
    </span>
  );
}
