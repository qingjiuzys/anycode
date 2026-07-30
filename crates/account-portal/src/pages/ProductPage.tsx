import { Link } from "react-router-dom";
import { ProductArchDiagram } from "../components/ProductArchDiagram";
import { useLocale, useT } from "../i18n/context";
import { DESKTOP_DOWNLOAD_URL } from "../lib/desktopDownload";
import { useAuth } from "../hooks/useAuth";
import { SITE_PATHS } from "@anycode/site-urls";

const COPY = {
  zh: {
    kicker: "SYSTEM ARCHITECTURE",
    title: "本地是默认，云端是选项",
    body: "anyCode 是可扩展的本地 Agent 工作台：项目、工具、Skills 与审批在本机执行；需要更强推理时，再按需接入 Cloud Auto / Agnes Chat。",
    localCore: "LOCAL CORE",
    optionalCloud: "OPTIONAL CLOUD",
    localBoundary: "数据、工具与审批留在本机；离线也可跑通主路径。",
    cloudBoundary: "仅模型推理走托管网关；账号与额度在 anycode.work。",
    modelRouter: "本机 → 模型路由 → 云端（可选）",
    nativeMedia: "原生媒体",
    secureContext: "安全审批",
    pillarsKicker: "PRODUCT STACK",
    pillarsTitle: "产品边界",
    pillarsLead: "从桌面工作台到云端账号，每一层职责清晰。",
    pillars: [
      {
        code: "01",
        title: "Desktop Workbench",
        body: "anyCode.app 内嵌工作台，本机启动即可对话、跑任务与管理自动化。",
      },
      {
        code: "02",
        title: "Agent Runtime",
        body: "多轮 LLM + 工具循环、Skills、会话与审批同属本地编排核心。",
      },
      {
        code: "03",
        title: "Models",
        body: "可扩展本地 / BYOK 模型；云端仅开放 Cloud Auto 与 Agnes Chat。",
      },
      {
        code: "04",
        title: "Native Media",
        body: "macOS 上 Apple Speech、Vision OCR、本地 TTS 与 Keychain 凭据。",
      },
      {
        code: "05",
        title: "Channels & Daemon",
        body: "微信 / Telegram / Discord 与定时任务可由桌面或 anycode-daemon 承载。",
      },
      {
        code: "06",
        title: "Cloud Account",
        body: "Portal 负责登录、套餐与设备关联；不改变本地执行默认路径。",
      },
    ],
    staysKicker: "DATA BOUNDARY",
    staysTitle: "什么留在本地",
    stays: [
      "项目文件、工具输出与会话上下文默认不上传",
      "敏感命令与文件写入需本地审批后才执行",
      "云端请求仅在你启用托管模型时发生",
    ],
  },
  en: {
    kicker: "SYSTEM ARCHITECTURE",
    title: "Local by default. Cloud by choice.",
    body: "anyCode is an extensible local Agent workbench: projects, tools, skills, and approvals run on device. Cloud Auto / Agnes Chat join only when you need more inference.",
    localCore: "LOCAL CORE",
    optionalCloud: "OPTIONAL CLOUD",
    localBoundary: "Data, tools, and approvals stay on device — the main path works offline.",
    cloudBoundary: "Only model inference hits the gateway; account & quota live on anycode.work.",
    modelRouter: "Device → model router → cloud (optional)",
    nativeMedia: "Native media",
    secureContext: "Approvals",
    pillarsKicker: "PRODUCT STACK",
    pillarsTitle: "Product boundary",
    pillarsLead: "From desktop workbench to cloud account — each layer has a clear job.",
    pillars: [
      {
        code: "01",
        title: "Desktop Workbench",
        body: "anyCode.app embeds the workbench — chat, tasks, and automations start locally.",
      },
      {
        code: "02",
        title: "Agent Runtime",
        body: "Multi-turn LLM + tool loop, skills, sessions, and approvals share one local core.",
      },
      {
        code: "03",
        title: "Models",
        body: "Extensible local / BYOK models; cloud surface is Cloud Auto and Agnes Chat only.",
      },
      {
        code: "04",
        title: "Native Media",
        body: "On macOS: Apple Speech, Vision OCR, local TTS, and Keychain credentials.",
      },
      {
        code: "05",
        title: "Channels & Daemon",
        body: "WeChat / Telegram / Discord and cron can run in Desktop or anycode-daemon.",
      },
      {
        code: "06",
        title: "Cloud Account",
        body: "Portal handles login, plans, and device linking — local execution stays the default.",
      },
    ],
    staysKicker: "DATA BOUNDARY",
    staysTitle: "What stays local",
    stays: [
      "Project files, tool output, and session context are not uploaded by default",
      "Sensitive shell and file writes wait for local approval",
      "Cloud requests happen only when you enable hosted models",
    ],
  },
} as const;

export function ProductPage() {
  const locale = useLocale();
  const t = useT();
  const { authenticated } = useAuth();
  const copy = COPY[locale];

  return (
    <div className="nx-site nx-site--product">
      <section className="nx-product-onepage" aria-labelledby="nx-product-title">
        <div className="nx-frame nx-product-onepage__inner">
          <div className="nx-product-onepage__intro">
            <header className="nx-page-hero">
              <p className="nx-kicker">{copy.kicker}</p>
              <h1 id="nx-product-title">{copy.title}</h1>
              <p className="nx-page-hero__lead">{copy.body}</p>
            </header>

            <ProductArchDiagram
              copy={{
                localCore: copy.localCore,
                optionalCloud: copy.optionalCloud,
                localBoundary: copy.localBoundary,
                cloudBoundary: copy.cloudBoundary,
                modelRouter: copy.modelRouter,
                nativeMedia: copy.nativeMedia,
                secureContext: copy.secureContext,
              }}
            />
          </div>

          <div className="nx-product-onepage__stack">
            <header className="nx-product-onepage__stack-head">
              <p className="nx-kicker">{copy.pillarsKicker}</p>
              <h2>{copy.pillarsTitle}</h2>
              <p>{copy.pillarsLead}</p>
            </header>

            <div className="nx-product__pillar-grid nx-product__pillar-grid--compact">
              {copy.pillars.map((item) => (
                <article className="nx-product__pillar" key={item.code}>
                  <span className="nx-product__pillar-code">{item.code}</span>
                  <h3>{item.title}</h3>
                  <p>{item.body}</p>
                </article>
              ))}
            </div>
          </div>

          <aside className="nx-product-onepage__cta">
            <div className="nx-product-onepage__stays">
              <p className="nx-kicker">{copy.staysKicker}</p>
              <h2>{copy.staysTitle}</h2>
              <ul>
                {copy.stays.map((line) => (
                  <li key={line}>{line}</li>
                ))}
              </ul>
            </div>

            <div className="nx-product-onepage__actions">
              <a className="nx-btn nx-btn--primary" href={DESKTOP_DOWNLOAD_URL}>
                {t("hero.ctaDownload")} <span aria-hidden>↓</span>
              </a>
              <Link className="nx-btn nx-btn--secondary" to={authenticated ? "/console" : "/register"}>
                {authenticated ? t("hero.ctaConsole") : t("hero.ctaGetStarted")}{" "}
                <span aria-hidden>→</span>
              </Link>
              <Link className="nx-text-link" to={SITE_PATHS.features}>
                {t("nav.features")}
              </Link>
            </div>
          </aside>
        </div>
      </section>
    </div>
  );
}
