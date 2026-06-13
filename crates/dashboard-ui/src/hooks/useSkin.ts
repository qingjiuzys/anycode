import { useEffect, useState } from "react";

export type Skin = "mono" | "indigo" | "coral" | "teal";

const STORAGE_KEY = "anycode-dashboard-skin";
const SKINS: Skin[] = ["mono", "indigo", "coral", "teal"];

export function applySkin(skin: Skin) {
  document.documentElement.dataset.skin = skin;
}

export function getSkin(): Skin {
  const saved = localStorage.getItem(STORAGE_KEY);
  if (saved && SKINS.includes(saved as Skin)) {
    return saved as Skin;
  }
  return "indigo";
}

export function setSkin(skin: Skin) {
  localStorage.setItem(STORAGE_KEY, skin);
  applySkin(skin);
}

export function useSkin() {
  const [skin, setSkinState] = useState<Skin>(() => getSkin());

  useEffect(() => {
    applySkin(skin);
  }, [skin]);

  const pick = (next: Skin) => {
    setSkin(next);
    setSkinState(next);
  };

  return { skin, setSkin: pick, skins: SKINS };
}
