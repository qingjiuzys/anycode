import { Link } from "react-router-dom";
import { CaseSlidePreview } from "../components/CaseSlidePreview";
import { CaseThumb } from "../components/CaseThumb";
import { Logo } from "../components/Logo";
import { useAuth } from "../hooks/useAuth";
import { useLocale, useT } from "../i18n/context";
import {
  caseCopy,
  casePath,
  featuredCase,
  gridCases,
  type CaseItemId,
} from "../lib/cases";
import { DESKTOP_DOWNLOAD_URL } from "../lib/desktopDownload";
import { SITE_PATHS } from "@anycode/site-urls";

const PAGE_COPY = {
  zh: {
    headline: "让每一个想法，进入自己的运行轨道。",
    lead: "以项目为中心组织模型、工具与产物，形成持续积累的个人工作系统。",
    features: ["项目上下文", "多模型协作", "持续记忆"],
    wbTitleLead: "与",
    wbTitleAccent: "智能体",
    wbTitleRest: "一起构建",
    wbSubtitle: "Agent 驱动的完整交付链",
    wbPlaceholder: "今天想构建什么？",
    wbConnected: "已连接工作台",
    wbProject: "产品发布",
    wbModel: "DeepSeek V4 Flash",
    wbNewAgent: "新建 Agent",
    wbSearch: "搜索",
    wbColleagues: "发现同事",
    wbProjects: "项目",
    entriesTitle: "继续探索",
    entriesLead: "特性、架构与套餐。",
    featuresTitle: "特性",
    featuresBody: "Agent、模型、审批与自动化。",
    productTitle: "产品",
    productBody: "本地是默认，云端是选项。",
    plansTitle: "套餐",
    plansBody: "本地免费，云端按需。",
    ctaTitle: "从一句话开始。",
    ctaBody: "下载 anyCode，在本机跑第一个任务。",
    qrCaption: "扫码加入用户群",
    qrAlt: "企业微信用户群二维码",
  },
  en: {
    headline: "Every idea finds its own orbit.",
    lead: "Organize models, tools, and deliverables around projects into a system that keeps compounding.",
    features: ["Project context", "Multi-model", "Persistent memory"],
    wbTitleLead: "Build with ",
    wbTitleAccent: "agents",
    wbTitleRest: "",
    wbSubtitle: "Agent-driven delivery chain",
    wbPlaceholder: "What do you want to build today?",
    wbConnected: "Workbench connected",
    wbProject: "Product launch",
    wbModel: "DeepSeek V4 Flash",
    wbNewAgent: "New Agent",
    wbSearch: "Search",
    wbColleagues: "Discover colleagues",
    wbProjects: "Projects",
    entriesTitle: "Explore",
    entriesLead: "Features, architecture, and plans.",
    featuresTitle: "Features",
    featuresBody: "Agent, models, approvals, automation.",
    productTitle: "Product",
    productBody: "Local by default. Cloud by choice.",
    plansTitle: "Plans",
    plansBody: "Local free. Cloud when needed.",
    ctaTitle: "Start with one sentence.",
    ctaBody: "Download anyCode and run your first local task.",
    qrCaption: "Scan to join community",
    qrAlt: "WeCom community QR code",
  },
} as const;

const ENTRIES = [
  { to: SITE_PATHS.features, titleKey: "featuresTitle" as const, bodyKey: "featuresBody" as const },
  { to: SITE_PATHS.product, titleKey: "productTitle" as const, bodyKey: "productBody" as const },
  { to: SITE_PATHS.plans, titleKey: "plansTitle" as const, bodyKey: "plansBody" as const },
] as const;

const WB_NAV = [
  { key: "wbNewAgent" as const, icon: "✎" },
  { key: "wbSearch" as const, icon: "⌕" },
  { key: "wbColleagues" as const, icon: "◎" },
];

function WorkbenchPreview({
  copy,
}: {
  copy: (typeof PAGE_COPY)["zh"] | (typeof PAGE_COPY)["en"];
}) {
  const projects = [copy.wbProject, "anycode", "skills-office"];
  return (
    <div className="orbit-wb" aria-hidden>
      <aside>
        <div className="orbit-wb__brand">
          <Logo size="sm" />
        </div>
        <span className="is-active">{copy.wbProjects === "项目" ? "今天" : "Today"}</span>
        <span>{copy.wbProjects}</span>
        <span>Skills</span>
      </aside>
      <div className="orbit-wb__main">
        <div className="orbit-wb__top">
          <span>{copy.wbProject}</span>
          <span>{copy.wbModel}</span>
        </div>
        <p className="orbit-wb__prompt">{copy.wbPlaceholder}</p>
        <div className="orbit-wb__activity">
          <span>{copy.wbConnected}</span>
          <span>{copy.wbSubtitle}</span>
        </div>
        <div className="orbit-wb__composer">
          {copy.wbPlaceholder} <b>↑</b>
        </div>
      </div>
    </div>
  );
}

export function HomePageOrbit() {
  const { authenticated } = useAuth();
  const locale = useLocale();
  const t = useT();
  const copy = PAGE_COPY[locale];
  const cases = caseCopy(locale);
  const featured = featuredCase();
  const featuredItem = cases.items[featured.id as CaseItemId];
  const cards = gridCases();
  const startTo = authenticated ? "/console" : "/register";

  return (
    <div className="orbit-home">
      <section className="orbit-hero" aria-labelledby="orbit-hero-title">
        <div className="orbit-frame orbit-hero__inner">
          <div className="orbit-hero__copy">
            <h1 id="orbit-hero-title">{copy.headline}</h1>
            <p className="orbit-hero__lead">{copy.lead}</p>
            <div className="orbit-hero__actions">
              <a className="orbit-btn orbit-btn--primary" href={DESKTOP_DOWNLOAD_URL}>
                {t("hero.ctaDownload")}
                <span aria-hidden>↓</span>
              </a>
              <Link className="orbit-btn orbit-btn--ghost" to={startTo}>
                {authenticated ? t("hero.ctaConsole") : t("hero.ctaGetStarted")}
                <span aria-hidden>→</span>
              </Link>
            </div>
            <ul className="orbit-hero__features">
              {copy.features.map((feature, index) => (
                <li key={feature}>
                  <span>0{index + 1}</span>
                  <strong>{feature}</strong>
                </li>
              ))}
            </ul>
          </div>
          <div className="orbit-hero__visual">
            <WorkbenchPreview copy={copy} />
            <aside className="orbit-hero__qr">
              <img
                src="/images/wecom-community-qr.png"
                alt={copy.qrAlt}
                width={112}
                height={112}
              />
              <span>{copy.qrCaption}</span>
            </aside>
          </div>
        </div>
      </section>

      <section className="orbit-cases" aria-labelledby="orbit-cases-title">
        <div className="orbit-frame">
          <header className="orbit-section-head">
            <div>
              <p className="orbit-kicker">{cases.sectionKicker}</p>
              <h2 id="orbit-cases-title">{cases.sectionTitle}</h2>
            </div>
            <p>{cases.sectionLead}</p>
          </header>

          <div className="orbit-case-feature">
            <CaseSlidePreview
              title={featuredItem.slideTitle}
              sub={featuredItem.slideSub}
              body={featuredItem.slideBody}
              strong={featuredItem.slideStrong}
              steps={featuredItem.slideSteps}
            />
            <div className="orbit-case-feature__copy">
              <span className="orbit-case-feature__tag">{featuredItem.tag}</span>
              <h3>{featuredItem.title}</h3>
              <blockquote>
                <span aria-hidden>&gt;</span>
                <p>{featuredItem.prompt}</p>
              </blockquote>
              <ul className="orbit-case-feature__meta">
                <li>
                  <span>{cases.modelLabel.toUpperCase()}</span>
                  <strong>{featured.model}</strong>
                </li>
                <li>
                  <span>{cases.skillLabel.toUpperCase()}</span>
                  <strong>{featured.skill}</strong>
                </li>
                <li>
                  <span>{cases.outputLabel.toUpperCase()}</span>
                  <strong>{featuredItem.output}</strong>
                </li>
              </ul>
              <div className="orbit-case-feature__actions">
                {featured.demoUrl ? (
                  <a
                    className="orbit-btn orbit-btn--primary"
                    href={featured.demoUrl}
                    target="_blank"
                    rel="noreferrer"
                  >
                    {cases.openDemo} <span aria-hidden>→</span>
                  </a>
                ) : null}
                <Link className="orbit-btn orbit-btn--ghost" to={casePath(featured.id)}>
                  {cases.viewCase}
                </Link>
              </div>
            </div>
          </div>

          <div className="orbit-cases__grid">
            {cards.map((card) => {
              const item = cases.items[card.id as CaseItemId];
              return (
                <Link className="orbit-case-card" key={card.id} to={casePath(card.id)}>
                  <div className="orbit-case-card__thumb">
                    <CaseThumb kind={card.kind} />
                  </div>
                  <h3>{item.title}</h3>
                  <p>{item.summary}</p>
                  <footer>
                    <span>{card.model}</span>
                    <span>{card.skill}</span>
                  </footer>
                </Link>
              );
            })}
          </div>
        </div>
      </section>

      <section className="orbit-entries" aria-labelledby="orbit-entries-title">
        <div className="orbit-frame">
          <header className="orbit-section-head">
            <div>
              <h2 id="orbit-entries-title">{copy.entriesTitle}</h2>
            </div>
            <p>{copy.entriesLead}</p>
          </header>
          <div className="orbit-entries__grid">
            {ENTRIES.map((entry) => (
              <Link className="orbit-entry" key={entry.to} to={entry.to}>
                <h3>{copy[entry.titleKey]}</h3>
                <p>{copy[entry.bodyKey]}</p>
                <span className="orbit-entry__go">
                  {t("common.viewDetails")} <span aria-hidden>→</span>
                </span>
              </Link>
            ))}
          </div>
        </div>
      </section>

      <section className="orbit-final">
        <div className="orbit-frame orbit-final__inner">
          <div>
            <h2>{copy.ctaTitle}</h2>
            <p>{copy.ctaBody}</p>
          </div>
          <div className="orbit-final__actions">
            <a className="orbit-btn orbit-btn--primary" href={DESKTOP_DOWNLOAD_URL}>
              {t("hero.ctaDownload")} <span aria-hidden>↓</span>
            </a>
            <Link className="orbit-btn orbit-btn--ghost" to={startTo}>
              {authenticated ? t("hero.ctaConsole") : t("hero.ctaGetStarted")}{" "}
              <span aria-hidden>→</span>
            </Link>
          </div>
        </div>
      </section>

      <footer className="orbit-footer">
        <div className="orbit-frame orbit-footer__inner">
          <div className="orbit-footer__brand">
            <Logo size="sm" />
            <strong>anyCode</strong>
          </div>
          <span>{t("footer.tagline")}</span>
        </div>
      </footer>
    </div>
  );
}
