import type { ReactNode } from "react";

type IconProps = {
  name: string;
  filled?: boolean;
  className?: string;
  size?: number;
};

const icons: Record<string, ReactNode> = {
  account_circle: (
    <>
      <circle cx="12" cy="8" r="3.25" />
      <path d="M5 20a7 7 0 0 1 14 0" />
    </>
  ),
  add: <path d="M12 5v14M5 12h14" />,
  arrow_upward: <path d="M12 19V5M5 12l7-7 7 7" />,
  analytics: (
    <>
      <path d="M4 19h16" />
      <path d="M7 16l3-4 3 2 4-6" />
    </>
  ),
  article: (
    <>
      <path d="M6 4h10l4 4v12H6z" />
      <path d="M14 4v4h4" />
      <path d="M9 13h6M9 17h6" />
    </>
  ),
  bar_chart: (
    <>
      <path d="M5 19V9" />
      <path d="M12 19V5" />
      <path d="M19 19v-7" />
    </>
  ),
  cancel: (
    <>
      <circle cx="12" cy="12" r="8" />
      <path d="m9 9 6 6M15 9l-6 6" />
    </>
  ),
  chat: (
    <>
      <path d="M5 6.5h14v9H9l-4 3z" />
      <path d="M8.5 10h7M8.5 13h4" />
    </>
  ),
  check_circle: (
    <>
      <circle cx="12" cy="12" r="8" />
      <path d="m8.5 12.5 2.25 2.25L15.75 9" />
    </>
  ),
  chevron_left: <path d="m14.5 6-6 6 6 6" />,
  chevron_right: <path d="m9.5 6 6 6-6 6" />,
  close: <path d="M6 6l12 12M18 6 6 18" />,
  construction: (
    <>
      <path d="M7 7 5.5 5.5a2.1 2.1 0 0 1 3-3L10 4" />
      <path d="m4 20 9.5-9.5" />
      <path d="m14 7 3 3 3-3-3-3z" />
    </>
  ),
  corporate_fare: (
    <>
      <path d="M4 21V5a2 2 0 0 1 2-2h8v18" />
      <path d="M14 9h4a2 2 0 0 1 2 2v10" />
      <path d="M8 7h2M8 11h2M8 15h2M17 13h.01M17 17h.01" />
    </>
  ),
  dashboard: (
    <>
      <rect x="4" y="4" width="7" height="7" rx="1.5" />
      <rect x="13" y="4" width="7" height="5" rx="1.5" />
      <rect x="13" y="11" width="7" height="9" rx="1.5" />
      <rect x="4" y="13" width="7" height="7" rx="1.5" />
    </>
  ),
  dashboard_customize: (
    <>
      <rect x="3" y="3" width="7" height="7" rx="1.5" />
      <rect x="14" y="3" width="7" height="5" rx="1.5" />
      <rect x="3" y="14" width="7" height="7" rx="1.5" />
      <path d="M14 17h7M14 20h5" />
    </>
  ),
  dark_mode: <path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z" />,
  description: (
    <>
      <path d="M7 3h7l4 4v14H7z" />
      <path d="M14 3v5h4M9 12h6M9 16h6" />
    </>
  ),
  dns: (
    <>
      <rect x="4" y="5" width="16" height="5" rx="1.5" />
      <rect x="4" y="14" width="16" height="5" rx="1.5" />
      <path d="M7 7.5h.01M7 16.5h.01M10 7.5h7M10 16.5h7" />
    </>
  ),
  download: <path d="M12 4v10m0 0 4-4m-4 4-4-4M5 20h14" />,
  error: (
    <>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 7.5v5M12 16.5h.01" />
    </>
  ),
  expand_less: <path d="m7 15 5-5 5 5" />,
  expand_more: <path d="m7 9 5 5 5-5" />,
  fact_check: (
    <>
      <path d="M4 5h16v14H4z" />
      <path d="m8 9 1.5 1.5L12 8M14 9h3M8 15h3M14 15h3" />
    </>
  ),
  filter_list: <path d="M4 7h16M7 12h10M10 17h4" />,
  folder: (
    <>
      <path d="M3.5 6.5h6l2 2H20a1 1 0 0 1 1 1v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-10a1 1 0 0 1 1-1z" />
    </>
  ),
  folder_off: (
    <>
      <path d="m4 4 16 16" />
      <path d="M3.5 6.5h4.75l2 2H20a1 1 0 0 1 1 1v7.75" />
      <path d="M18 19.5H5a2 2 0 0 1-2-2V8.25" />
    </>
  ),
  folder_open: (
    <>
      <path d="M3 8.5v-2h6l2 2h8a1 1 0 0 1 1 1v2" />
      <path d="M4 19.5h14.5l2-8H5.5z" />
    </>
  ),
  forum: (
    <>
      <path d="M4 5h12v8H8l-4 3z" />
      <path d="M10 15h6l4 3V9" />
    </>
  ),
  help_center: (
    <>
      <circle cx="12" cy="12" r="8" />
      <path d="M9.75 9a2.5 2.5 0 1 1 3.55 2.25c-.8.45-1.3.9-1.3 1.75M12 16.5h.01" />
    </>
  ),
  help_outline: (
    <>
      <circle cx="12" cy="12" r="8" />
      <path d="M9.75 9a2.5 2.5 0 1 1 3.55 2.25c-.8.45-1.3.9-1.3 1.75M12 16.5h.01" />
    </>
  ),
  history: (
    <>
      <path d="M12 8v4l2.5 1.5" />
      <path d="M12 20a8 8 0 1 0-8-8" />
      <path d="M4 4v5h5" />
    </>
  ),
  home: <path d="m4 11 8-7 8 7v9H6v-6h12" />,
  inventory: (
    <>
      <path d="M4 7h16v13H4z" />
      <path d="M7 4h10l3 3H4zM9 12h6" />
    </>
  ),
  inventory_2: (
    <>
      <path d="M4 7h16v13H4z" />
      <path d="M7 4h10l3 3H4zM9 12h6" />
    </>
  ),
  light_mode: (
    <>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2.5v2M12 19.5v2M4.6 4.6 6 6M18 18l1.4 1.4M2.5 12h2M19.5 12h2M4.6 19.4 6 18M18 6l1.4-1.4" />
    </>
  ),
  login: <path d="M10 7V5h9v14h-9v-2M4 12h10m0 0-3-3m3 3-3 3" />,
  logout: <path d="M14 7V5H5v14h9v-2M10 12h10m0 0-3-3m3 3-3 3" />,
  notifications: (
    <>
      <path d="M6 17h12l-1.5-2.5V11a4.5 4.5 0 0 0-9 0v3.5z" />
      <path d="M10 19a2 2 0 0 0 4 0" />
    </>
  ),
  policy: (
    <>
      <path d="M12 3 5 6v5c0 4.5 3 7.5 7 10 4-2.5 7-5.5 7-10V6z" />
      <path d="m9 12 2 2 4-5" />
    </>
  ),
  psychology: (
    <>
      <path d="M9 18H8a4 4 0 0 1-1-7.87A5.5 5.5 0 0 1 17.5 8a4.5 4.5 0 0 1-1.5 8.74V21h-6v-3" />
      <path d="M10 9h.01M14 9h.01M10 13h4" />
    </>
  ),
  radar: (
    <>
      <circle cx="12" cy="12" r="8" />
      <circle cx="12" cy="12" r="3" />
      <path d="M12 12 18 6" />
    </>
  ),
  rate_review: (
    <>
      <path d="M5 5h14v10H9l-4 4z" />
      <path d="M9 9h6M9 12h4" />
    </>
  ),
  refresh: <path d="M20 6v5h-5M4 18v-5h5M19 11a7 7 0 0 0-12-4.9M5 13a7 7 0 0 0 12 4.9" />,
  robot_2: (
    <>
      <rect x="6" y="8" width="12" height="9" rx="2" />
      <path d="M12 8V4M9 4h6M8.5 12h.01M15.5 12h.01M10 16h4M4 11v3M20 11v3" />
    </>
  ),
  schedule: (
    <>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 7.5V12l3 2" />
    </>
  ),
  search: (
    <>
      <circle cx="10.5" cy="10.5" r="5.5" />
      <path d="m15.5 15.5 4 4" />
    </>
  ),
  send: <path d="M4 4l17 8-17 8 3-8zM7 12h8" />,
  settings: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M19 12a7 7 0 0 0-.08-1l2-1.55-2-3.46-2.35.95a7.5 7.5 0 0 0-1.73-1L14.5 3h-5l-.34 2.94a7.5 7.5 0 0 0-1.73 1L5.08 6l-2 3.46 2 1.55a7 7 0 0 0 0 2l-2 1.55 2 3.46 2.35-.95a7.5 7.5 0 0 0 1.73 1L9.5 21h5l.34-2.94a7.5 7.5 0 0 0 1.73-1l2.35.95 2-3.46-2-1.55c.05-.33.08-.66.08-1z" />
    </>
  ),
  settings_suggest: (
    <>
      <circle cx="11" cy="12" r="3" />
      <path d="M17.5 7.5 19 6M17.5 16.5 19 18M20 12h2M4 12h2M6.5 7.5 5 6M6.5 16.5 5 18" />
      <path d="M15.5 12a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0z" />
    </>
  ),
  sync: <path d="M20 7v5h-5M4 17v-5h5M19 12a7 7 0 0 0-12-5M5 12a7 7 0 0 0 12 5" />,
  terminal: <path d="m5 7 5 5-5 5M12 17h7" />,
  verified: (
    <>
      <path d="M12 3 5 6v5c0 4.5 3 7.5 7 10 4-2.5 7-5.5 7-10V6z" />
      <path d="m9 12 2 2 4-5" />
    </>
  ),
  verified_user: (
    <>
      <path d="M12 3 5 6v5c0 4.5 3 7.5 7 10 4-2.5 7-5.5 7-10V6z" />
      <path d="m9 12 2 2 4-5" />
    </>
  ),
  warning: (
    <>
      <path d="M12 4 3.5 19h17z" />
      <path d="M12 9v4M12 16.5h.01" />
    </>
  ),
};

export function Icon({ name, filled, className, size = 20 }: IconProps) {
  const icon = icons[name] ?? <circle cx="12" cy="12" r="3" />;

  return (
    <svg
      className={`dw-icon select-none shrink-0 ${filled ? "fill" : ""} ${className ?? ""}`}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={filled ? 2.4 : 2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      focusable="false"
    >
      {icon}
    </svg>
  );
}
