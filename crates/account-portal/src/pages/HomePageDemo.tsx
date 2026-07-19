import { Link } from "react-router-dom";
import { Logo } from "../components/Logo";
import { useAuth } from "../hooks/useAuth";
import { useLocale, useT } from "../i18n/context";
import { DESKTOP_DOWNLOAD_URL } from "../lib/desktopDownload";
import { SITE_PATHS } from "@anycode/site-urls";

const PAGE_COPY = {
  zh: {
    runtime: "本地 Agent，真正开始工作",
    entriesTitle: "继续探索",
    entriesLead: "特性、产品架构与套餐各自成页，首页只保留入口。",
    featuresTitle: "特性星系",
    featuresBody: "每个星球是一项核心能力。",
    productTitle: "产品架构",
    productBody: "本地是默认，云端是选项。",
    plansTitle: "套餐定价",
    plansBody: "本地免费，云端按需。",
    ctaTitle: "让下一项工作，从一句话开始。",
    ctaBody: "下载 anyCode，在本机运行第一个 Agent 任务。",
    footerStatus: "算法备案审核中 · 受邀内测",
  },
  en: {
    runtime: "Local agents that actually get work done",
    entriesTitle: "Explore further",
    entriesLead: "Features, product architecture, and plans each have their own page.",
    featuresTitle: "Capability galaxy",
    featuresBody: "Each planet is a core capability.",
    productTitle: "Product architecture",
    productBody: "Local by default. Cloud by choice.",
    plansTitle: "Plans & pricing",
    plansBody: "Local free. Cloud when you need it.",
    ctaTitle: "Start the next piece of work with one sentence.",
    ctaBody: "Download anyCode and run your first Agent task locally.",
    footerStatus: "Algorithm filing under review · Invite-only preview",
  },
} as const;

const ENTRIES = [
  { to: SITE_PATHS.features, titleKey: "featuresTitle", bodyKey: "featuresBody", code: "01" },
  { to: SITE_PATHS.product, titleKey: "productTitle", bodyKey: "productBody", code: "02" },
  { to: SITE_PATHS.plans, titleKey: "plansTitle", bodyKey: "plansBody", code: "03" },
] as const;

export function HomePageDemo() {
  const { authenticated } = useAuth();
  const locale = useLocale();
  const t = useT();
  const copy = PAGE_COPY[locale];

  return (
    <div className="nx-home">
      <section className="nx-hero" aria-labelledby="nx-hero-title">
        <div className="nx-hero__rail nx-hero__rail--one" aria-hidden />
        <div className="nx-hero__rail nx-hero__rail--two" aria-hidden />
        <div className="nx-hero__rail nx-hero__rail--three" aria-hidden />

        <div className="nx-frame">
          <div className="nx-hero__status">
            <span className="nx-live-dot" aria-hidden />
            <span>{t("hero.reviewStatus")}</span>
            <span className="nx-hero__status-tech">RUST RUNTIME / macOS NATIVE / LOCAL-FIRST</span>
          </div>

          <div className="nx-hero__copy">
            <p className="nx-kicker">{t("hero.eyebrow")}</p>
            <h1 id="nx-hero-title">anyCode</h1>
            <p className="nx-hero__statement">{copy.runtime}</p>
            <p className="nx-hero__body">{t("hero.subtitle")}</p>
            <div className="nx-hero__actions">
              <a className="nx-btn nx-btn--primary" href={DESKTOP_DOWNLOAD_URL}>
                {t("hero.ctaDownload")}
                <span aria-hidden>↓</span>
              </a>
              <Link className="nx-btn nx-btn--secondary" to={authenticated ? "/console" : "/register"}>
                {authenticated ? t("hero.ctaConsole") : t("hero.ctaGetStarted")}
                <span aria-hidden>→</span>
              </Link>
              {!authenticated ? (
                <Link className="nx-text-link" to="/login">
                  {t("hero.ctaSignIn")}
                </Link>
              ) : null}
            </div>
          </div>

          <div className="nx-workspace" aria-label="LIVE WORKSPACE">
            <div className="nx-workspace__bar">
              <div className="nx-window-dots" aria-hidden>
                <i />
                <i />
                <i />
              </div>
              <span>anyCode / LIVE WORKSPACE</span>
              <span className="nx-workspace__health">
                <i aria-hidden /> {locale === "zh" ? "本地运行正常" : "Local runtime healthy"}
              </span>
            </div>

            <div className="nx-workspace__body">
              <aside className="nx-workspace__sidebar">
                <div className="nx-workspace__brand">
                  <Logo size="sm" />
                  <span>anyCode</span>
                </div>
                <span className="nx-workspace__label">PROJECT</span>
                <strong>{locale === "zh" ? "产品发布" : "Product launch"}</strong>
                <div className="nx-workspace__nav">
                  <span className="is-active">01 / Agent</span>
                  <span>02 / Files</span>
                  <span>03 / Assets</span>
                </div>
              </aside>

              <div className="nx-workspace__mission">
                <div className="nx-mission__head">
                  <span>{locale === "zh" ? "执行链" : "Execution"} / 03 STEPS</span>
                  <span className="nx-status-chip">{locale === "zh" ? "运行中" : "Running"}</span>
                </div>
                <h2>
                  {locale === "zh"
                    ? "为产品发布构建完整交付包"
                    : "Build the complete delivery kit for a product launch"}
                </h2>
                <p>
                  {locale === "zh"
                    ? "Agent 正在读取项目、调用 Skills 并验证结果"
                    : "Agent is reading the project, invoking Skills, and verifying outputs"}
                </p>

                <div className="nx-mission__steps">
                  <div className="is-done">
                    <span>01</span>
                    <strong>{locale === "zh" ? "理解任务" : "Understand"}</strong>
                    <i aria-hidden />
                  </div>
                  <div className="is-active">
                    <span>02</span>
                    <strong>{locale === "zh" ? "执行交付" : "Execute"}</strong>
                    <i aria-hidden />
                  </div>
                  <div>
                    <span>03</span>
                    <strong>{locale === "zh" ? "验证结果" : "Verify"}</strong>
                    <i aria-hidden />
                  </div>
                </div>

                <div className="nx-mission__prompt">
                  <span>&gt;</span>
                  <p>{t("hero.demoPromptUser")}</p>
                  <button type="button" aria-label={t("hero.ctaGetStarted")}>
                    ↑
                  </button>
                </div>
              </div>

              <aside className="nx-workspace__models">
                <span className="nx-workspace__label">MODEL ROUTER</span>
                <div className="nx-model is-active">
                  <span>LOCAL</span>
                  <strong>On device</strong>
                </div>
                <div className="nx-model">
                  <span>AUTO</span>
                  <strong>Cloud Auto</strong>
                </div>
                <div className="nx-model">
                  <span>CHAT</span>
                  <strong>Agnes Chat</strong>
                </div>
              </aside>
            </div>
          </div>

          <div className="nx-hero__signals" aria-label={t("features.eyebrow")}>
            <span>
              <i className="is-cyan" /> LOCAL AGENT
            </span>
            <span>
              <i className="is-lime" /> EXTENSIBLE MODELS
            </span>
            <span>
              <i className="is-coral" /> NATIVE MEDIA
            </span>
            <span>
              <i className="is-violet" /> OPTIONAL CLOUD
            </span>
          </div>
        </div>
      </section>

      <section className="nx-home-entries" aria-labelledby="nx-entries-title">
        <div className="nx-frame">
          <header className="nx-section-head">
            <div>
              <p className="nx-kicker">NEXT / EXPLORE</p>
              <h2 id="nx-entries-title">{copy.entriesTitle}</h2>
            </div>
            <p>{copy.entriesLead}</p>
          </header>
          <div className="nx-home-entries__grid">
            {ENTRIES.map((entry) => (
              <Link className="nx-home-entry" key={entry.to} to={entry.to}>
                <span className="nx-home-entry__code">{entry.code}</span>
                <h3>{copy[entry.titleKey]}</h3>
                <p>{copy[entry.bodyKey]}</p>
                <span className="nx-home-entry__go">
                  {t("common.viewDetails")} <span aria-hidden>→</span>
                </span>
              </Link>
            ))}
          </div>
        </div>
      </section>

      <section className="nx-final">
        <div className="nx-frame nx-final__inner">
          <div>
            <p className="nx-kicker">START LOCAL</p>
            <h2>{copy.ctaTitle}</h2>
            <p>{copy.ctaBody}</p>
          </div>
          <div className="nx-final__actions">
            <a className="nx-btn nx-btn--primary" href={DESKTOP_DOWNLOAD_URL}>
              {t("hero.ctaDownload")} <span aria-hidden>↓</span>
            </a>
            <Link className="nx-btn nx-btn--secondary" to={authenticated ? "/console" : "/register"}>
              {authenticated ? t("hero.ctaConsole") : t("hero.ctaGetStarted")} <span aria-hidden>→</span>
            </Link>
          </div>
        </div>
      </section>

      <footer className="nx-footer">
        <div className="nx-frame nx-footer__inner">
          <div className="nx-footer__brand">
            <Logo size="sm" />
            <strong>anyCode</strong>
          </div>
          <span>{t("footer.tagline")}</span>
          <span>{copy.footerStatus}</span>
        </div>
      </footer>
    </div>
  );
}
