export type ChangelogSectionKind = "added" | "changed" | "fixed";

export type LocalizedText = {
  zh: string;
  en: string;
};

export type ChangelogSection = {
  kind: ChangelogSectionKind;
  items: LocalizedText[];
};

export type ChangelogRelease = {
  version: string;
  date: string;
  tag: string;
  summary?: LocalizedText;
  sections: ChangelogSection[];
};

/** Newest first — rendered as a vertical timeline (top → bottom). */
export const CHANGELOG_RELEASES: ChangelogRelease[] = [
  {
    version: "0.3.1",
    date: "2026-07-30",
    tag: "v0.3.1",
    summary: {
      zh: "交付物预览增强、门户产品/下载页改版与用户文档重写。",
      en: "Deliverable previews, refreshed portal product/download pages, and rewritten user docs.",
    },
    sections: [
      {
        kind: "added",
        items: [
          {
            zh: "交付物卡片：表格/CSV/XLSX 缩略图、Office/PDF HTML 预览、PPT 幻灯片网格、GFM 表格与 Mermaid 图块。",
            en: "Deliverable cards: spreadsheet thumbnails, HTML sidecars for Office/PDF, PPT slide grids, inline GFM tables, and Mermaid blocks.",
          },
          {
            zh: "Skill 输出：`anycode-ppt` 与 Office starters 通过 `ANYCODE_ARTIFACT` 附带 `preview_path`。",
            en: "Skill emit: `anycode-ppt` and office starters emit `ANYCODE_ARTIFACT` with `preview_path`.",
          },
          {
            zh: "门户更新日志页：按版本上下排列的时间线视图。",
            en: "Portal changelog page with a vertical release timeline.",
          },
        ],
      },
      {
        kind: "changed",
        items: [
          {
            zh: "统一交付物查看器（`DeliverableCompactShell`、`selectDeliverableViewer`）；工作台与卡片共享路由。",
            en: "Consolidated deliverable viewers (`DeliverableCompactShell`, `selectDeliverableViewer`); workbench and cards share routing.",
          },
          {
            zh: "用户文档 L0–L3 解析层（中英文交付物指南）并补充界面截图。",
            en: "User docs L0–L3 parsing layers (zh/en deliverables guides) with refreshed screenshots.",
          },
          {
            zh: "产品页单屏左右布局 + SVG 架构图；下载页左侧文案、右侧下载卡片。",
            en: "Product page single-viewport layout with SVG architecture diagram; downloads page copy-left / card-right.",
          },
        ],
      },
      {
        kind: "fixed",
        items: [
          {
            zh: "剥离 artifact 标记后保留助手 anyCode 产品回声。",
            en: "Assistant anyCode product echo preserved when artifact markers are stripped.",
          },
          {
            zh: "补注册缺失的 `web` / `table_chart` 图标。",
            en: "Missing `web` / `table_chart` icon registrations.",
          },
        ],
      },
    ],
  },
  {
    version: "0.3.0",
    date: "2026-07-01",
    tag: "v0.3.0",
    summary: {
      zh: "拷问模式、团队云端交接、Office skills 与营销站点首版。",
      en: "Grill mode, cloud team handoff, Office skills, and first marketing site.",
    },
    sections: [
      {
        kind: "added",
        items: [
          {
            zh: "Grill Me（拷问模式）：实现前一次只问一个 `AskUserQuestion`。",
            en: "Grill Me: one `AskUserQuestion` at a time before implementation.",
          },
          {
            zh: "团队交接：LAN mDNS + account-service 云端 A2A 流式中继（Portal Team 页）。",
            en: "Team handoff: LAN mDNS plus cloud A2A streaming relay on account-service (Portal Team page).",
          },
          {
            zh: "Office skills：`anycode-docx` / `anycode-ppt` / `anycode-xlsx` / `anycode-pdf` starters。",
            en: "Office skills: `anycode-docx` / `anycode-ppt` / `anycode-xlsx` / `anycode-pdf` starter skills.",
          },
          {
            zh: "营销站点：首页 Workbench 预览、产品架构、特性星系、套餐与下载页。",
            en: "Marketing site: Workbench preview, product architecture, features galaxy, plans, and downloads.",
          },
          {
            zh: "macOS 原生媒体层：STT、OCR、TTS、UserNotifications、Keychain 等。",
            en: "macOS native media layer: STT, OCR, TTS, UserNotifications, Keychain, and more.",
          },
        ],
      },
      {
        kind: "changed",
        items: [
          {
            zh: "GitHub Release 在 tag 上仅分发 macOS `.dmg`；Linux/Windows 需手动 workflow。",
            en: "GitHub Release on tag ships macOS `.dmg` only; Linux/Windows are manual workflow_dispatch.",
          },
        ],
      },
    ],
  },
  {
    version: "0.2.3",
    date: "2026-06-15",
    tag: "v0.2.3",
    sections: [
      {
        kind: "fixed",
        items: [
          {
            zh: "桌面 Release CI：未配置 Apple 密钥时跳过公证，避免空 Team ID 报错。",
            en: "Desktop release CI skips notarization when Apple secrets are unset.",
          },
        ],
      },
    ],
  },
  {
    version: "0.2.2",
    date: "2026-06-01",
    tag: "v0.2.2",
    sections: [
      {
        kind: "changed",
        items: [
          {
            zh: "macOS GitHub Release 仅 `.dmg`（CLI 内置于 anyCode.app）。",
            en: "macOS GitHub Release ships `.dmg` only with CLI bundled in anyCode.app.",
          },
          {
            zh: "无 Developer ID 时桌面 CI 使用 ad-hoc 签名。",
            en: "Desktop CI uses ad-hoc codesign when Apple Developer ID secrets are absent.",
          },
        ],
      },
    ],
  },
  {
    version: "0.2.0",
    date: "2026-05-01",
    tag: "v0.2.0",
    summary: {
      zh: "Digital Workbench 控制面、渠道 cron、MCP/LSP 加固与模型目录扩展。",
      en: "Digital Workbench control plane, channel cron, MCP/LSP hardening, and model catalog expansion.",
    },
    sections: [
      {
        kind: "added",
        items: [
          {
            zh: "Digital Workbench V3 控制面闭环与 Playwright e2e。",
            en: "Digital Workbench V3 control plane closure and Playwright e2e.",
          },
          {
            zh: "微信桥接增强、macOS Apple Speech STT / Vision OCR。",
            en: "WeChat bridge improvements and macOS Apple Speech STT / Vision OCR.",
          },
        ],
      },
    ],
  },
];
