import { Link } from "react-router-dom";
import { HelpContact } from "./HelpContact";
import { docsPageHref, type DocsLocale } from "./catalog";

export function DocsHelp({ locale }: { locale: DocsLocale }) {
  const isZh = locale === "zh";
  return (
    <article className="docs-article">
      <h1>{isZh ? "帮助与支持" : "Help & support"}</h1>
      <p>
        {isZh
          ? "如需产品协助，请通过邮件或用户群联系我们。"
          : "Need a hand with anyCode? Reach us by email or join the user group below."}
      </p>
      <HelpContact locale={locale} />
      <p>
        {isZh ? "常见问题：" : "Common fixes: "}
        <Link to={docsPageHref(locale, "guide/troubleshooting")}>
          {isZh ? "排错指南" : "Troubleshooting"}
        </Link>
      </p>
    </article>
  );
}
