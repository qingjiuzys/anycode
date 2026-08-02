/** Console sidebar mark — cloud orbit, distinct from top-nav brand logo. */
export function ConsoleSidebarMark({ size = 32 }: { size?: number }) {
  return (
    <svg
      className="nx-console-sidebar__mark"
      width={size}
      height={size}
      viewBox="0 0 32 32"
      fill="none"
      aria-hidden
    >
      <circle cx="16" cy="16" r="13.5" stroke="currentColor" strokeWidth="1.1" opacity="0.38" />
      <circle cx="16" cy="16" r="8.5" stroke="currentColor" strokeWidth="0.8" opacity="0.22" />
      <path
        fill="currentColor"
        d="M11.2 19.4h9.8a3.6 3.6 0 0 0 .28-7.18 4.6 4.6 0 0 0-8.95 1.08A3.2 3.2 0 0 0 11.2 19.4Z"
        opacity="0.92"
      />
      <circle cx="16" cy="15.8" r="2.2" fill="currentColor" />
    </svg>
  );
}
