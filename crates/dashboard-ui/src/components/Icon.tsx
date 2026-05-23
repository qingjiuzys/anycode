export function Icon({
  name,
  filled,
  className,
  size = 20,
}: {
  name: string;
  filled?: boolean;
  className?: string;
  size?: number;
}) {
  return (
    <span
      className={`material-symbols-outlined select-none leading-none shrink-0 ${filled ? "fill" : ""} ${className ?? ""}`}
      style={{
        fontSize: size,
        fontFamily: '"Material Symbols Outlined"',
        fontVariationSettings: filled
          ? '"FILL" 1, "wght" 400, "GRAD" 0, "opsz" 24'
          : '"FILL" 0, "wght" 400, "GRAD" 0, "opsz" 24',
      }}
      aria-hidden
    >
      {name}
    </span>
  );
}
