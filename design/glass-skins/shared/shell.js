(function () {
  const SKIN_KEY = "anycode-dashboard-skin";
  const THEME_KEY = "anycode-dashboard-theme";
  const SKINS = ["mono", "indigo", "coral", "teal"];

  function getSkin() {
    const s = localStorage.getItem(SKIN_KEY);
    return SKINS.includes(s) ? s : "indigo";
  }

  function getTheme() {
    const t = localStorage.getItem(THEME_KEY);
    if (t === "light" || t === "dark") return t;
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }

  function applySkin(skin) {
    document.documentElement.dataset.skin = skin;
    localStorage.setItem(SKIN_KEY, skin);
    document.querySelectorAll("[data-skin-btn]").forEach((btn) => {
      btn.setAttribute("aria-pressed", btn.dataset.skinBtn === skin ? "true" : "false");
    });
    const status = document.getElementById("hub-status");
    if (status) status.textContent = "Skin: " + skin + " · Theme: " + getTheme();
  }

  function applyTheme(theme) {
    document.documentElement.classList.toggle("dark", theme === "dark");
    document.documentElement.dataset.theme = theme;
    localStorage.setItem(THEME_KEY, theme);
    const btn = document.getElementById("theme-toggle");
    if (btn) btn.textContent = theme === "dark" ? "Light" : "Dark";
    const status = document.getElementById("hub-status");
    if (status) status.textContent = "Skin: " + getSkin() + " · Theme: " + theme;
  }

  function init() {
    applySkin(getSkin());
    applyTheme(getTheme());

    document.querySelectorAll("[data-skin-btn]").forEach((btn) => {
      btn.addEventListener("click", () => applySkin(btn.dataset.skinBtn));
    });

    const themeBtn = document.getElementById("theme-toggle");
    if (themeBtn) {
      themeBtn.addEventListener("click", () => {
        applyTheme(getTheme() === "dark" ? "light" : "dark");
      });
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
