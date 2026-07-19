import { Link } from "react-router-dom";
import { docsPageHref, type DocsLocale } from "./catalog";

export function DocsHome({ locale }: { locale: DocsLocale }) {
  const isZh = locale === "zh";
  return (
    <div className="docs-home">
      <p className="docs-home__eyebrow">{isZh ? "Documentation" : "Documentation"}</p>
      <h1>{isZh ? "从本机开始" : "Start on your machine"}</h1>
      <p className="docs-home__tagline">
        {isZh
          ? "桌面工作台、守护进程、微信桥与 BYOK 模型目录——本地执行，云端可选。"
          : "Desktop workbench, daemon, WeChat bridge, and BYOK catalog — execute locally, cloud optional."}
      </p>
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
          <strong>{isZh ? "BYOK 模型目录" : "BYOK model catalog"}</strong>
          <span>
            {isZh
              ? "DeepSeek、GLM、Anthropic、Ollama 等，本地配置与探测。"
              : "DeepSeek, GLM, Anthropic, Ollama, and more — configure and probe locally."}
          </span>
        </li>
        <li>
          <strong>{isZh ? "个人微信桥" : "Personal WeChat bridge"}</strong>
          <span>
            {isZh
              ? "扫码绑定后在手机上发任务、审批工具、接收文件。"
              : "Scan to bind — send tasks, approve tools, and receive files from your phone."}
          </span>
        </li>
        <li>
          <strong>{isZh ? "自动化与审批" : "Automation & approvals"}</strong>
          <span>
            {isZh
              ? "自然语言 cron、运行历史，敏感操作始终需要确认。"
              : "Natural-language cron and run history — sensitive actions always ask first."}
          </span>
        </li>
      </ul>
    </div>
  );
}
