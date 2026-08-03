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
    version: "0.39.0",
    date: "2026-08-03",
    tag: "v0.39.0",
    summary: {
      zh: "Plan tree 会话持久化 + Build 审阅、Git 栏、Workbench 图标 Dock、桌面全窗口拖拽。",
      en: "Session plan tree + Build review, Git bar, workbench icon dock, and full-window desktop drag.",
    },
    sections: [
      {
        kind: "added",
        items: [
          {
            zh: "Plan tree：计划/待办写入 SQLite，Plan 面板展示；Build 审阅条（选模型后再执行）。",
            en: "Plan tree: plans/todos in SQLite with Plan panel and Build review before execution.",
          },
          {
            zh: "Git 栏：Composer 上方显示 +/- 变更与 Commit & Push。",
            en: "Git bar above composer with change stats and Commit & Push.",
          },
          {
            zh: "Workbench 图标 Dock + 桌面窗口全区域拖拽。",
            en: "Workbench header icon dock and draggable frameless desktop window.",
          },
        ],
      },
      {
        kind: "changed",
        items: [
          {
            zh: "Workbench 打开时隐藏 transcript、收窄 Composer；Pause/Send 合并。",
            en: "Hide transcript when workbench opens; narrower composer; merged Pause/Send.",
          },
          {
            zh: "Browser 侧栏仅在新 Browser 工具调用时自动打开。",
            en: "Browser panel auto-opens only on new live Browser tool calls.",
          },
        ],
      },
      {
        kind: "fixed",
        items: [
          {
            zh: "会话标题与 Workbench 图标重叠；Browser 误触发自动打开。",
            en: "Session title vs icon overlap; spurious browser panel auto-open.",
          },
        ],
      },
    ],
  },
  {
    version: "0.35.0",
    date: "2026-08-02",
    tag: "v0.35.0",
    summary: {
      zh: "记忆系统 auto-memory：受限后台提取 + 四阶段 consolidation、MEMORY.md 索引、本地引擎回退。",
      en: "Memory auto-memory (automem): sandboxed background extraction + four-phase consolidation, MEMORY.md index, local-engine fallback.",
    },
    sections: [
      {
        kind: "added",
        items: [
          {
            zh: "新增 automem 学习管线：fork 受限后台代理提取会话，四阶段（orient→gather→consolidate→prune）生成记忆。",
            en: "New automem pipeline: forked sandboxed agent extracts sessions and consolidates memory in four phases.",
          },
          {
            zh: "MEMORY.md 作为记忆索引入口（≤200 行 / ≤25KB），互斥锁 + cursor 保证并发安全。",
            en: "MEMORY.md as a bounded index entry (≤200 lines / ≤25KB) with mutex + cursor safety.",
          },
          {
            zh: "autoDream 时间/会话门控 + 锁；LLM 不可用时自动回退到本地 dedup/promote/forget + 向量检索。",
            en: "autoDream time/session gating with locking; automatic fallback to the local dedup/promote/forget engine when no LLM is available.",
          },
        ],
      },
      {
        kind: "changed",
        items: [
          {
            zh: "automem 支持配置开关（enabled / fork_agent / 门控阈值 / 转录上限）。",
            en: "automem is configurable (enabled / fork_agent / gating thresholds / transcript cap).",
          },
        ],
      },
    ],
  },
  {
    version: "0.3.4",
    date: "2026-08-01",
    tag: "v0.3.4",
    summary: {
      zh: "内置 Skills 重组：bake-off 晋升、Office 脚本外置、验证技能并存。",
      en: "Built-in skills refresh: bake-off promotions, shared Office builders, dual verification skills.",
    },
    sections: [
      {
        kind: "added",
        items: [
          {
            zh: "晋升一批前端/文档/设计类 starter skill，并纳入 verification-before-completion 与 internal-comms。",
            en: "Promoted frontend/docs/design starter skills, plus verification-before-completion and internal-comms.",
          },
          {
            zh: "Office 构建脚本统一到 scripts/office/，供 anycode-docx / xlsx / ppt 复用。",
            en: "Shared Office builders under scripts/office/ for anycode-docx / xlsx / ppt.",
          },
        ],
      },
      {
        kind: "changed",
        items: [
          {
            zh: "移除重复交付类 starter；英文日报/周报改走 internal-comms；中文 cn-* 保留。",
            en: "Removed overlapping delivery starters; EN daily/weekly briefs use internal-comms; keep Chinese cn-*.",
          },
        ],
      },
    ],
  },
  {
    version: "0.3.3",
    date: "2026-07-31",
    tag: "v0.3.3",
    summary: {
      zh: "文本大脑可贴图（OCR 底层）、助手朗读（TTS）、课件演示与气泡不再露出 OCR 原文。",
      en: "Text brains can attach images via OCR, Speak via TTS, courseware demo, OCR hidden from bubbles.",
    },
    sections: [
      {
        kind: "added",
        items: [
          {
            zh: "文本对话模型（如 DeepSeek Flash）可贴图：发送时走 Apple OCR，识别结果只给模型。",
            en: "Text-only chat (e.g. DeepSeek Flash) can attach images; Apple OCR runs on send for the model only.",
          },
          {
            zh: "助手气泡支持朗读（走 TTS 能力槽，不要求 chat 本身是 TTS）。",
            en: "Assistant bubbles can Speak via the TTS capability slot (chat need not be a TTS model).",
          },
          {
            zh: "门户课件演示：县城咖啡市场调研 8 页 FDE 翻页稿。",
            en: "Portal courseware demo: county-coffee market-research 8-page FDE deck.",
          },
        ],
      },
      {
        kind: "changed",
        items: [
          {
            zh: "用户气泡显示原文 + 图片缩略图，不再把 OCR 全文铺进会话。",
            en: "User bubbles show original text + image thumbnails; OCR is no longer dumped into the transcript.",
          },
        ],
      },
    ],
  },
  {
    version: "0.3.2",
    date: "2026-07-31",
    tag: "v0.3.2",
    summary: {
      zh: "套餐配额调整、门户案例可直接打开演示、首页支持 /拷问 /目标。",
      en: "Updated plan quotas, openable portal case demos, and home slash modes.",
    },
    sections: [
      {
        kind: "added",
        items: [
          {
            zh: "门户交付案例：四个案例均可「打开演示」（PPT 翻页 / 周报 / 经营表预览）。",
            en: "Portal cases: all four demos are openable in-browser (PPT decks, weekly report, ops sheet).",
          },
          {
            zh: "Workbench 首页 composer 支持 `/拷问`、`/目标` 斜杠模式（与会话页一致）。",
            en: "Workbench home composer supports `/拷问` and `/目标` slash modes (same as conversation).",
          },
        ],
      },
      {
        kind: "changed",
        items: [
          {
            zh: "Free：新用户赠送 2000 万 tokens（DeepSeek Flash 托管）。",
            en: "Free: new users get 20M bonus hosted tokens (DeepSeek Flash).",
          },
          {
            zh: "Cloud 5h：维持 5 小时滚动窗口、窗口内 1000 次调用。",
            en: "Cloud 5h: unchanged — 1,000 calls per 5-hour rolling window.",
          },
          {
            zh: "Pro（¥599/月）：改为 5 小时滚动窗口、窗口内 10000 次调用；DeepSeek V4 Pro 暂未开放。",
            en: "Pro (¥599/mo): 10,000 calls per 5-hour rolling window; DeepSeek V4 Pro temporarily unavailable.",
          },
          {
            zh: "移除设置页「局域网协作」入口（同事交接走云端 A2A）。",
            en: "Removed Settings → LAN collaboration (team handoff remains cloud A2A).",
          },
        ],
      },
    ],
  },
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
