import { Link } from "@tanstack/react-router";
import brandIcon from "@/assets/brand-icon.png";
import { useT } from "@/i18n/context";

export function BrandMark({
  size = "md",
  showTitle = false,
  linked = false,
  variant = "sidebar",
}: {
  size?: "sm" | "md";
  showTitle?: boolean;
  linked?: boolean;
  variant?: "sidebar" | "login";
}) {
  const t = useT();
  const iconClass = size === "sm" ? "dw-brand-mark dw-brand-mark--sm" : "dw-brand-mark";
  const titleClass =
    variant === "login" ? "text-xl font-bold text-on-surface truncate" : "dw-brand-mark__title";

  const content = (
    <>
      <img src={brandIcon} alt="" className={iconClass} />
      {showTitle && <span className={titleClass}>{t("layout.brand")}</span>}
    </>
  );

  if (linked) {
    return (
      <Link to="/" className="dw-brand-mark__link no-underline">
        {content}
      </Link>
    );
  }

  return <div className="dw-brand-mark__wrap">{content}</div>;
}
