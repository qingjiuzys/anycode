import { useSkin, type Skin } from "@/hooks/useSkin";
import { useLocale, useT } from "@/i18n/context";

const SKIN_LABELS: Record<Skin, { zh: string; en: string }> = {
  mono: { zh: "无彩色", en: "Mono" },
  indigo: { zh: "蓝紫", en: "Indigo" },
  coral: { zh: "珊瑚", en: "Coral" },
  teal: { zh: "青绿", en: "Teal" },
};

function Swatch({ skin, pressed, onPick }: { skin: Skin; pressed: boolean; onPick: () => void }) {
  return (
    <button
      type="button"
      className={`skin-swatch skin-swatch--${skin}`}
      aria-pressed={pressed}
      aria-label={skin}
      onClick={onPick}
    />
  );
}

/** Compact row for topbar. */
export function SkinPickerCompact() {
  const { skin, setSkin, skins } = useSkin();
  return (
    <div className="skin-picker-compact flex items-center gap-1" role="group" aria-label="Skin">
      {skins.map((id) => (
        <Swatch key={id} skin={id} pressed={skin === id} onPick={() => setSkin(id)} />
      ))}
    </div>
  );
}

/** Settings grid with labels. */
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
            className={`skin-preview-card glass-card glass-card--static text-left w-full border-0 cursor-pointer ${
              selected ? "skin-preview-card--selected" : ""
            }`}
            onClick={() => setSkin(id)}
          >
            <strong className="text-sm">{SKIN_LABELS[id][locale]}</strong>
            <p className="text-xs text-secondary m-0 mt-1">{t(`settings.skin.${id}`)}</p>
            <div className="skin-preview-mini mt-3 flex gap-2 items-center">
              <div className="skin-preview-mini-bar" />
              <div
                className="skin-preview-mini-dot"
                style={id === "mono" ? { background: "#fff" } : undefined}
              />
            </div>
          </button>
        );
      })}
    </div>
  );
}
