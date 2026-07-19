import { LogoMarkIcon } from "./LogoMarkIcon";

type LogoSize = "sm" | "md" | "lg";

const sizePx: Record<LogoSize, number> = {
  sm: 32,
  md: 40,
  lg: 52,
};

export function Logo({
  size = "md",
  alt = "anyCode",
}: {
  size?: LogoSize;
  alt?: string;
}) {
  const px = sizePx[size];
  return (
    <span className={`logo logo-${size}`} role="img" aria-label={alt}>
      <LogoMarkIcon size={px} />
    </span>
  );
}
