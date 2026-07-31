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
    title: "开源 · 企业自主可控 Agent",
    lead: "Harness · Grill Me 拷问 · 会话交接 · BYOK 本地执行",
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
    footerStatus: "算法备案审核中 · 受邀内测",
    qrCaption: "扫码加入用户群",
    qrAlt: "企业微信用户群二维码",
  },
  en: {
    title: "Open-source agents you control",
    lead: "Harness · Grill Me · session handoff · BYOK local runtime",
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
    footerStatus: "Algorithm filing under review · Invite-only",
    qrCaption: "Scan to join community",
    qrAlt: "WeCom community QR code",
  },
} as const;

const ENTRIES = [
  { to: SITE_PATHS.features, titleKey: "featuresTitle" as const, bodyKey: "featuresBody" as const, code: "01" },
  { to: SITE_PATHS.product, titleKey: "productTitle" as const, bodyKey: "productBody" as const, code: "02" },
  { to: SITE_PATHS.plans, titleKey: "plansTitle" as const, bodyKey: "plansBody" as const, code: "03" },
] as const;

const WB_NAV = [
  { key: "wbNewAgent" as const, icon: "✎" },
  { key: "wbSearch" as const, icon: "⌕" },
  { key: "wbColleagues" as const, icon: "◎" },
];

function WorkbenchMini({
  copy,
}: {
  copy: (typeof PAGE_COPY)["zh"] | (typeof PAGE_COPY)["en"];
}) {
  const projects = [copy.wbProject, "anycode", "skills-office"];
  return (
    <div className="nx-wb-mini" aria-hidden>
      <aside className="nx-wb-mini__sidebar">
        <div className="nx-wb-mini__brand">
          <Logo size="sm" />
          <strong>anyCode</strong>
        </div>
        <nav className="nx-wb-mini__nav">
          {WB_NAV.map((item) => (
            <span key={item.key}>
              <i>{item.icon}</i>
              {copy[item.key]}
            </span>
          ))}
        </nav>
        <div className="nx-wb-mini__section">{copy.wbProjects}</div>
        <ul className="nx-wb-mini__projects">
          {projects.map((name, i) => (
            <li key={name} className={i === 0 ? "is-active" : undefined}>
              <i aria-hidden>▢</i>
              {name}
            </li>
          ))}
        </ul>
      </aside>
      <div className="nx-wb-mini__main">
        <div className="nx-wb-mini__glow" />
        <h2>
          {copy.wbTitleLead}
          <span>{copy.wbTitleAccent}</span>
          {copy.wbTitleRest}
        </h2>
        <p>{copy.wbSubtitle}</p>
        <div className="nx-wb-mini__composer">
          <div className="nx-wb-mini__placeholder">{copy.wbPlaceholder}</div>
          <div className="nx-wb-mini__toolbar">
            <span>{copy.wbProject}</span>
            <span>{copy.wbModel}</span>
            <i>↑</i>
          </div>
        </div>
        <div className="nx-wb-mini__meta">
          <i />
          {copy.wbConnected}
        </div>
      </div>
    </div>
  );
}

export function HomePageDemo() {
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
    <div className="nx-home">
      <section className="nx-hero nx-hero--slim" aria-labelledby="nx-hero-title">
        <div className="nx-frame nx-hero__slim">
          <div className="nx-hero__copy">
            <p className="nx-kicker">{t("hero.eyebrow")}</p>
            <h1 id="nx-hero-title">anyCode</h1>
            <p className="nx-hero__headline">{copy.title}</p>
            <p className="nx-hero__lead">{copy.lead}</p>
            <div className="nx-hero__actions">
              <a className="nx-btn nx-btn--primary" href={DESKTOP_DOWNLOAD_URL}>
                {t("hero.ctaDownload")}
                <span aria-hidden>↓</span>
              </a>
              <Link className="nx-btn nx-btn--secondary" to={startTo}>
                {authenticated ? t("hero.ctaConsole") : t("hero.ctaGetStarted")}
                <span aria-hidden>→</span>
              </Link>
            </div>
          </div>
          <div className="nx-hero__visual">
            <WorkbenchMini copy={copy} />
            <aside className="nx-hero__qr">
              <img
                src="/images/wecom-community-qr.png"
                alt={copy.qrAlt}
                width={48}
                height={48}
              />
              <span>{copy.qrCaption}</span>
            </aside>
          </div>
        </div>
      </section>

      <section className="nx-cases" aria-labelledby="nx-cases-title">
        <div className="nx-frame">
          <header className="nx-section-head">
            <div>
              <p className="nx-kicker">{cases.sectionKicker}</p>
              <h2 id="nx-cases-title">{cases.sectionTitle}</h2>
            </div>
            <p>{cases.sectionLead}</p>
          </header>

          <div className="nx-case-feature">
            <CaseSlidePreview
              title={featuredItem.slideTitle}
              sub={featuredItem.slideSub}
              body={featuredItem.slideBody}
              strong={featuredItem.slideStrong}
              steps={featuredItem.slideSteps}
            />
            <div className="nx-case-feature__copy">
              <span className="nx-case-feature__tag">{featuredItem.tag}</span>
              <h3>{featuredItem.title}</h3>
              <blockquote>
                <span aria-hidden>&gt;</span>
                <p>{featuredItem.prompt}</p>
              </blockquote>
              <ul className="nx-case-feature__meta">
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
              <div className="nx-case-feature__actions">
                {featured.demoUrl ? (
                  <a className="nx-btn nx-btn--primary" href={featured.demoUrl} target="_blank" rel="noreferrer">
                    {cases.openDemo} <span aria-hidden>→</span>
                  </a>
                ) : null}
                <Link className="nx-btn nx-btn--ghost" to={casePath(featured.id)}>
                  {cases.viewCase}
                </Link>
              </div>
            </div>
          </div>

          <div className="nx-cases__grid nx-cases__grid--loose">
            {cards.map((card, index) => {
              const item = cases.items[card.id as CaseItemId];
              return (
                <Link className="nx-case-card" key={card.id} to={casePath(card.id)}>
                  <div className="nx-case-card__thumb">
                    <CaseThumb kind={card.kind} />
                  </div>
                  <span className="nx-case-card__code">0{index + 1}</span>
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

      <section className="nx-home-entries" aria-labelledby="nx-entries-title">
        <div className="nx-frame">
          <header className="nx-section-head">
            <div>
              <p className="nx-kicker">EXPLORE</p>
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
            <p className="nx-kicker">START</p>
            <h2>{copy.ctaTitle}</h2>
            <p>{copy.ctaBody}</p>
          </div>
          <div className="nx-final__actions">
            <a className="nx-btn nx-btn--primary" href={DESKTOP_DOWNLOAD_URL}>
              {t("hero.ctaDownload")} <span aria-hidden>↓</span>
            </a>
            <Link className="nx-btn nx-btn--secondary" to={startTo}>
              {authenticated ? t("hero.ctaConsole") : t("hero.ctaGetStarted")}{" "}
              <span aria-hidden>→</span>
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
