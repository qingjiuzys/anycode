import { Link } from "react-router-dom";
import { docsPageHref, type DocsLocale } from "./catalog";

export function DocsHome({ locale }: { locale: DocsLocale }) {
  const isZh = locale === "zh";
  return (
    <div className="docs-home">
      <p className="docs-home__eyebrow">{isZh ? "Documentation" : "Documentation"}</p>
      <h1>{isZh ? "企业级开源 Agent · 独立部署" : "Enterprise Agent · Self-hosted"}</h1>
      <p className="docs-home__tagline">
        {isZh
          ? "开源免费（MIT），在本机或内网安装运行。配图说明安装、配置与工作台用法。"
          : "Free & open source (MIT). Install on your machine or private network — guides with screenshots."}
      </p>
      <figure className="docs-home__hero">
        <img
          src="/docs/assets/screenshots/home.png"
          alt={
            isZh
              ? "anyCode 企业级 Agent 工作台 — 支持独立部署"
              : "anyCode enterprise Agent workbench — self-hosted"
          }
          width={920}
          height={575}
          loading="eager"
        />
        <figcaption>
          {isZh
            ? "本地 Digital Workbench — 项目、会话、交付物与审批"
            : "Local Digital Workbench — projects, sessions, deliverables, approvals"}
        </figcaption>
      </figure>
      <div className="docs-home__actions">
        <Link className="nx-btn nx-btn--primary" to={docsPageHref(locale, "guide/getting-started")}>
          {isZh ? "快速开始" : "Quick start"} <span aria-hidden>→</span>
        </Link>
        <Link className="nx-btn nx-btn--secondary" to={docsPageHref(locale, "guide/workbench")}>
          {isZh ? "工作台导览" : "Workbench tour"}
        </Link>
      </div>
      <ul className="docs-home__features">
        <li>
          <strong>{isZh ? "配图快速开始" : "Screenshot walkthroughs"}</strong>
          <span>
            {isZh
              ? "安装、设置向导、侧栏页面——对照截图操作。"
              : "Install, setup wizard, and sidebar pages — follow along with screenshots."}
          </span>
        </li>
        <li>
          <strong>{isZh ? "BYOK 模型" : "BYOK models"}</strong>
          <span>
            {isZh
              ? "DeepSeek、GLM、Anthropic、Ollama 等，密钥只保存在本机。"
              : "DeepSeek, GLM, Anthropic, Ollama, and more — keys stay on your machine."}
          </span>
        </li>
        <li>
          <strong>{isZh ? "交付物与自动化" : "Deliverables & automation"}</strong>
          <span>
            {isZh
              ? "PDF、表格、幻灯片在对话里预览；自然语言创建定时任务。"
              : "Preview PDFs, spreadsheets, and slides in chat; schedule jobs in plain language."}
          </span>
        </li>
      </ul>
    </div>
  );
}
