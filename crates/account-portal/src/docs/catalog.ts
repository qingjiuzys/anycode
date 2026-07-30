export type DocsLocale = "en" | "zh";

export type DocsNavItem = {
  text: string;
  slug: string;
};

export type DocsNavSection = {
  text: string;
  items: DocsNavItem[];
};

const enSections: DocsNavSection[] = [
  {
    text: "Get started",
    items: [
      { text: "Quick start", slug: "guide/getting-started" },
      { text: "Install", slug: "guide/install" },
      { text: "Open the Workbench", slug: "guide/dashboard" },
    ],
  },
  {
    text: "Digital Workbench",
    items: [
      { text: "Workbench tour", slug: "guide/workbench" },
      { text: "Conversation deliverables", slug: "guide/deliverables" },
    ],
  },
  {
    text: "Desktop & automation",
    items: [
      { text: "Desktop app (macOS)", slug: "guide/desktop" },
      { text: "Headless daemon", slug: "guide/daemon" },
      { text: "Scheduled reminders", slug: "guide/cli-scheduler" },
      { text: "Common issues", slug: "guide/troubleshooting" },
    ],
  },
  {
    text: "Settings & skills",
    items: [
      { text: "Models", slug: "guide/models" },
      { text: "Config & security", slug: "guide/config-security" },
      { text: "Official skills catalog", slug: "guide/skills/index" },
      { text: "Memory", slug: "guide/memory" },
      { text: "Notifications", slug: "guide/notifications" },
    ],
  },
  {
    text: "Reference",
    items: [
      { text: "Agents & Skills", slug: "guide/agents" },
      { text: "Architecture", slug: "guide/architecture" },
      { text: "Development", slug: "guide/development" },
      { text: "All pages (index)", slug: "guide/hubs" },
    ],
  },
];

const zhSections: DocsNavSection[] = [
  {
    text: "开始使用",
    items: [
      { text: "快速开始", slug: "guide/getting-started" },
      { text: "安装", slug: "guide/install" },
      { text: "打开工作台", slug: "guide/dashboard" },
    ],
  },
  {
    text: "数字工作台",
    items: [
      { text: "工作台导览", slug: "guide/workbench" },
      { text: "会话交付物", slug: "guide/deliverables" },
    ],
  },
  {
    text: "桌面与自动化",
    items: [
      { text: "桌面应用（macOS）", slug: "guide/desktop" },
      { text: "无头守护进程", slug: "guide/daemon" },
      { text: "定时提醒", slug: "guide/cli-scheduler" },
      { text: "常见问题", slug: "guide/troubleshooting" },
    ],
  },
  {
    text: "设置与技能",
    items: [
      { text: "模型与端点", slug: "guide/models" },
      { text: "配置与安全", slug: "guide/config-security" },
      { text: "官方 Skills 目录", slug: "guide/skills/index" },
      { text: "记忆", slug: "guide/memory" },
      { text: "会话通知", slug: "guide/notifications" },
    ],
  },
  {
    text: "参考",
    items: [
      { text: "Agent 与 Skills", slug: "guide/agents" },
      { text: "架构说明", slug: "guide/architecture" },
      { text: "开发与贡献", slug: "guide/development" },
      { text: "全量索引", slug: "guide/hubs" },
    ],
  },
];

export function docsSections(locale: DocsLocale): DocsNavSection[] {
  return locale === "zh" ? zhSections : enSections;
}

/** Docs URLs are locale-agnostic; language comes from the site locale switcher. */
export function docsBase(_locale?: DocsLocale): string {
  return "/docs";
}

export function docsPageHref(_locale: DocsLocale, slug: string): string {
  return `/docs/${slug}`;
}

/** Strip optional legacy `/docs/zh` prefix and return the doc slug (may be empty). */
export function parseDocsSlug(pathname: string): string {
  let rest = pathname.replace(/^\/docs\/?/, "");
  if (rest === "zh" || rest.startsWith("zh/")) {
    rest = rest.slice(2).replace(/^\//, "");
  }
  return rest.replace(/\/$/, "");
}
