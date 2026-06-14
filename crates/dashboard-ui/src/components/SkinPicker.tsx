import type { CSSProperties } from "react";
import { Icon } from "@/components/Icon";
import { useSkin, type Skin } from "@/hooks/useSkin";
import { useLocale, useT } from "@/i18n/context";

export const SKIN_LABELS: Record<Skin, { zh: string; en: string }> = {
  mono: { zh: "无彩色", en: "Mono" },
  indigo: { zh: "蓝紫", en: "Indigo" },
  coral: { zh: "珊瑚", en: "Coral" },
  teal: { zh: "青绿", en: "Teal" },
};

export const SKIN_ACCENTS: Record<Skin, { accent: string; muted: string }> = {
  mono: { accent: "#ffffff", muted: "rgba(255, 255, 255, 0.12)" },
  indigo: { accent: "#6e6bff", muted: "rgba(110, 107, 255, 0.18)" },
  coral: { accent: "#e8826b", muted: "rgba(232, 130, 107, 0.16)" },
  teal: { accent: "#2dd4bf", muted: "rgba(45, 212, 191, 0.14)" },
};

export function SkinSwatch({
  skin,
  pressed,
  onPick,
  size = "md",
  showCheck = true,
}: {
  skin: Skin;
  pressed: boolean;
  onPick: () => void;
  size?: "sm" | "md";
  showCheck?: boolean;
}) {
  const locale = useLocale().startsWith("zh") ? "zh" : "en";
  return (
    <button
      type="button"
      className={`skin-swatch skin-swatch--${skin} skin-swatch--${size}`}
      aria-pressed={pressed}
      aria-label={SKIN_LABELS[skin][locale]}
      onClick={onPick}
    >
      {pressed && showCheck && (
        <Icon name="check" size={size === "sm" ? 12 : 14} className="skin-swatch__check" />
      )}
    </button>
  );
}

function SkinPreviewWindow({ skin }: { skin: Skin }) {
  const colors = SKIN_ACCENTS[skin];
  return (
    <div
      className="skin-preview-window"
      style={
        {
          "--skin-preview-accent": colors.accent,
          "--skin-preview-muted": colors.muted,
        } as CSSProperties
      }
    >
      <div className="skin-preview-window__sidebar">
        <span className="skin-preview-window__nav-line" />
        <span className="skin-preview-window__nav-line skin-preview-window__nav-line--active" />
        <span className="skin-preview-window__nav-line" />
      </div>
      <div className="skin-preview-window__main">
        <span className="skin-preview-window__topbar" />
        <span className="skin-preview-window__content">
          <span className="skin-preview-window__accent-pill" />
          <span className="skin-preview-window__line" />
          <span className="skin-preview-window__line skin-preview-window__line--short" />
        </span>
      </div>
    </div>
  );
}

/** Compact row for popovers and compact surfaces. */
export function SkinPickerCompact() {
  const { skin, setSkin, skins } = useSkin();
  return (
    <div className="skin-picker-compact" role="group" aria-label="Skin">
      {skins.map((id) => (
        <SkinSwatch
          key={id}
          skin={id}
          pressed={skin === id}
          onPick={() => setSkin(id)}
          size="sm"
        />
      ))}
    </div>
  );
}

/** Settings grid with labels and window preview. */
export function SkinPickerPanel() {
  const { skin, setSkin, skins } = useSkin();
  const t = useT();
  const locale = useLocale().startsWith("zh") ? "zh" : "en";

  return (
    <div className="skin-picker-grid">
      {skins.map((id) => {
        const selected = skin === id;
        return (
          <button
            key={id}
            type="button"
            className={`skin-preview-card ${selected ? "skin-preview-card--selected" : ""}`}
            onClick={() => setSkin(id)}
            aria-pressed={selected}
          >
            {selected && (
              <span className="skin-preview-card__badge" aria-hidden>
                <Icon name="check" size={14} />
              </span>
            )}
            <div className="skin-preview-card__header">
              <strong className="skin-preview-card__title">{SKIN_LABELS[id][locale]}</strong>
              <p className="skin-preview-card__subtitle">{t(`settings.skin.${id}`)}</p>
            </div>
            <SkinPreviewWindow skin={id} />
          </button>
        );
      })}
    </div>
  );
}
