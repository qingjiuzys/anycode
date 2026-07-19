import markUrl from "../../../../brand/anycode-mark.svg";

/** Canonical A monogram, bundled directly from brand/anycode-mark.svg. */
export function LogoMarkIcon({ size = 32 }: { size?: number }) {
  return (
    <img
      src={markUrl}
      alt=""
      width={size}
      height={size}
      aria-hidden="true"
      className="dw-logo-mark-icon"
    />
  );
}
