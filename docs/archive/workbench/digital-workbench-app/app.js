const current = document.body.dataset.page;

document.querySelectorAll("[data-nav]").forEach((item) => {
  item.classList.toggle("active", item.dataset.nav === current);
});

const theme = localStorage.getItem("anycode.workbench.theme");
if (theme === "dark") {
  document.body.classList.add("dark");
}

document.querySelectorAll("[data-theme-toggle]").forEach((button) => {
  button.addEventListener("click", () => {
    document.body.classList.toggle("dark");
    localStorage.setItem(
      "anycode.workbench.theme",
      document.body.classList.contains("dark") ? "dark" : "light",
    );
  });
});

document.querySelectorAll("[data-login]").forEach((button) => {
  button.addEventListener("click", () => {
    window.location.href = "login.html";
  });
});

document.querySelectorAll("[data-save]").forEach((button) => {
  button.addEventListener("click", () => {
    button.textContent = "已保存";
    window.setTimeout(() => {
      button.textContent = button.dataset.save || "保存";
    }, 1200);
  });
});
