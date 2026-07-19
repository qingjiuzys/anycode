import { useState } from "react";
import { Icon } from "@/components/Icon";
import { isTauriDesktop, pickDirectory } from "@/lib/desktopShell";

type Props = {
  id?: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  required?: boolean;
  autoFocus?: boolean;
  disabled?: boolean;
  "aria-label": string;
  /** Label for the browse button (desktop). */
  browseLabel?: string;
};

/** Path input with optional native folder picker (Tauri desktop). */
export function DirectoryPathField({
  id,
  value,
  onChange,
  placeholder,
  required,
  autoFocus,
  disabled = false,
  "aria-label": ariaLabel,
  browseLabel = "选择目录",
}: Props) {
  const [picking, setPicking] = useState(false);
  const canBrowse = isTauriDesktop() && !disabled;

  const onBrowse = async () => {
    if (!canBrowse || picking) return;
    setPicking(true);
    try {
      const path = await pickDirectory();
      if (path) onChange(path);
    } finally {
      setPicking(false);
    }
  };

  return (
    <div className={`dw-directory-path${disabled ? " dw-directory-path--disabled" : ""}`}>
      <button
        type="button"
        className="dw-directory-path__icon"
        onClick={() => void onBrowse()}
        disabled={!canBrowse || picking}
        aria-label={browseLabel}
        title={canBrowse ? browseLabel : undefined}
      >
        <Icon name="folder_open" size={18} />
      </button>
      <input
        id={id}
        className="dw-directory-path__input"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        required={required}
        autoFocus={autoFocus}
        disabled={disabled}
        aria-label={ariaLabel}
        title={value.trim() || undefined}
        spellCheck={false}
        autoComplete="off"
        dir="ltr"
      />
      {canBrowse && (
        <button
          type="button"
          className="dw-directory-path__browse"
          onClick={() => void onBrowse()}
          disabled={picking}
        >
          {picking ? "…" : browseLabel}
        </button>
      )}
    </div>
  );
}
