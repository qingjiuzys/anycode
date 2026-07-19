import { Link } from "react-router-dom";
import type { DocsLocale } from "./catalog";
import { docsPageHref } from "./catalog";
import { SITE_EMAILS } from "@anycode/site-urls";

export function HelpContact({ locale }: { locale: DocsLocale }) {
  const isZh = locale === "zh";
  const supportEmail = SITE_EMAILS.support;
  const copy = isZh
    ? {
        emailTitle: "邮箱支持",
        emailHint: "产品使用、账号与订阅问题，请发送邮件，我们会在 1–2 个工作日内回复。",
        qrTitle: "用户交流群",
        qrCaption: "微信扫码加入 anyCode 用户群",
        qrHint: "交流使用技巧、获取更新通知，与团队和其他用户互动。",
        docsLabel: "文档中心",
        docsHint: "安装、工作台、渠道桥接等完整指南。",
      }
    : {
        emailTitle: "Email support",
        emailHint:
          "For product usage, accounts, and billing questions. We aim to reply within 1–2 business days.",
        qrTitle: "Community group",
        qrCaption: "Scan with WeChat to join the anyCode user group",
        qrHint: "Share tips, get release news, and connect with the team and other users.",
        docsLabel: "Documentation",
        docsHint: "Full guides for install, Workbench, channel bridges, and more.",
      };

  return (
    <section className="docs-help-contact">
      <div className="docs-help-contact__card">
        <h2>{copy.emailTitle}</h2>
        <p>{copy.emailHint}</p>
        <a href={`mailto:${supportEmail}`}>{supportEmail}</a>
      </div>
      <div className="docs-help-contact__card">
        <h2>{copy.qrTitle}</h2>
        <figure>
          <img src="/images/community-qr.svg" alt={copy.qrCaption} width={220} height={220} />
          <figcaption>{copy.qrCaption}</figcaption>
        </figure>
        <p>{copy.qrHint}</p>
      </div>
      <div className="docs-help-contact__card docs-help-contact__card--wide">
        <h2>{copy.docsLabel}</h2>
        <p>{copy.docsHint}</p>
        <Link className="lx-btn lx-btn--ghost" to={docsPageHref(locale, "guide/getting-started")}>
          {copy.docsLabel}
        </Link>
      </div>
    </section>
  );
}
