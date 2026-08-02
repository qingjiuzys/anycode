export type CaseKind = "ppt" | "doc" | "sheet";

export type CaseLocale = "zh" | "en";

export type CaseDef = {
  id: string;
  kind: CaseKind;
  skill: string;
  model: string;
  featured?: boolean;
  /** Static demo under portal `public/demos/` — openable without Desktop. */
  demoUrl?: string;
};

export const CASES: CaseDef[] = [
  {
    id: "launch-ppt",
    kind: "ppt",
    skill: "anycode-ppt",
    model: "DeepSeek Flash",
    featured: true,
    demoUrl: "/demos/launch-ppt/index.html",
  },
  {
    id: "courseware",
    kind: "ppt",
    skill: "anycode-ppt",
    model: "DeepSeek Flash",
    demoUrl: "/demos/courseware/index.html",
  },
  {
    id: "weekly-report",
    kind: "doc",
    skill: "anycode-docx",
    model: "DeepSeek Flash",
    demoUrl: "/demos/weekly-report/index.html",
  },
  {
    id: "ops-sheet",
    kind: "sheet",
    skill: "anycode-xlsx",
    model: "DeepSeek Flash",
    demoUrl: "/demos/ops-sheet/index.html",
  },
];

const COPY = {
  zh: {
    sectionKicker: "CASES",
    sectionTitle: "交付案例",
    sectionLead: "一句话启动，本机落盘可验收文件。",
    viewCase: "查看案例",
    openDemo: "打开演示",
    backHome: "返回首页",
    promptLabel: "提示词",
    modelLabel: "模型",
    skillLabel: "Skill",
    outputLabel: "产物",
    stepsLabel: "怎么跑通",
    tryLabel: "自己试一次",
    tryBody: "先打开在线演示感受产物；再下载 anyCode，用同一句话在本机生成。",
    items: {
      "launch-ppt": {
        tag: "HTML PPT",
        title: "产品发布会演示稿",
        summary: "12 页发布稿：封面、架构、KPI、路线图、行动清单。",
        prompt:
          "帮我做一份 12 页产品发布 PPT：封面、架构、KPI、路线图、行动清单，FDE 编辑风。",
        output: "分页 HTML + 预览器，浏览器直接演示",
        slideTitle: "产品发布",
        slideSub: "本地 Agent · 可交付",
        slideBody: "一句话启动，Skills 落地",
        slideStrong: "可演示的幻灯片。",
        slideSteps: ["封面", "架构", "KPI", "行动"],
        steps: [
          "在 Workbench 输入提示词，可选 DeepSeek Flash",
          "Agent 调用 anycode-ppt，从模板复制并改文案",
          "run slides/ 校验并生成 index.html 预览器",
          "本机打开，← → 翻页演示",
        ],
      },
      courseware: {
        tag: "HTML PPT",
        title: "市场调研培训课件",
        summary: "县城咖啡案例：方法五步、圈层 KPI、误区对比、行动清单。8 页 FDE 编辑风。",
        prompt:
          "做一份县城咖啡市场调研培训课件：封面、Agenda、五步方法、圈层 KPI、问卷堆砌 vs 决策抽样、金句结论、行动清单、小结。FDE 编辑风，调用 anycode-ppt。",
        output: "8 页 HTML 课件 + index 预览器",
        slideTitle: "方法",
        slideSub: "五步链路",
        slideBody: "问题 → 样本 → 采集 → 交叉 →",
        slideStrong: "决策。",
        slideSteps: ["封面", "方法", "数据", "行动"],
        steps: [
          "描述受众与决策问题（开店 / 换址 / 不做）",
          "anycode-ppt 复制 cover / ladder / metrics / duo / checklist 模板",
          "run slides/ 校验视觉契约并生成 index.html",
          "本机打开预览，← → 翻页授课",
        ],
      },
      "weekly-report": {
        tag: "DOCX",
        title: "周报 / 正式报告",
        summary: "结构化 Word：背景、进展、风险、下周计划。",
        prompt: "写一份本周工作周报：背景、进展、风险、下周计划，正式公文语气。",
        output: "可编辑 .docx",
        slideTitle: "周报",
        slideSub: "结构化正文",
        slideBody: "标题层级清晰，段落可直接",
        slideStrong: "提交审阅。",
        slideSteps: ["背景", "进展", "风险", "计划"],
        steps: [
          "给出时间范围与项目上下文",
          "Agent 调用 anycode-docx 套模板",
          "本机生成并打开 Word",
          "按需改数字后发出",
        ],
      },
      "ops-sheet": {
        tag: "XLSX",
        title: "经营数据表",
        summary: "KPI + 多 sheet 工作簿语感，公式可审计、毛利率趋势一眼可见。",
        prompt: "做一份经营数据表：收入、成本、毛利，含公式与跨 sheet 汇总。",
        output: "可复核 .xlsx",
        slideTitle: "经营表",
        slideSub: "公式可审计",
        slideBody: "KPI 与跨 sheet 联动，数字",
        slideStrong: "可当场复核。",
        slideSteps: ["KPI", "收入", "成本", "汇总"],
        steps: [
          "说明指标与时间粒度",
          "Agent 调用 anycode-xlsx 建表",
          "写入公式与校验规则",
          "本机打开 Excel / Numbers 复核",
        ],
      },
    },
  },
  en: {
    sectionKicker: "CASES",
    sectionTitle: "Deliverable cases",
    sectionLead: "One sentence in. Shippable files out — on your machine.",
    viewCase: "View case",
    openDemo: "Open demo",
    backHome: "Back home",
    promptLabel: "Prompt",
    modelLabel: "Model",
    skillLabel: "Skill",
    outputLabel: "Output",
    stepsLabel: "How it runs",
    tryLabel: "Try it yourself",
    tryBody: "Open the online demo first, then download anyCode and run the same prompt locally.",
    items: {
      "launch-ppt": {
        tag: "HTML PPT",
        title: "Product launch deck",
        summary: "12-page launch deck: cover, architecture, KPIs, roadmap, checklist.",
        prompt:
          "Build a 12-page launch deck: cover, architecture, KPIs, roadmap, checklist — FDE editorial style.",
        output: "Paginated HTML + viewer",
        slideTitle: "Product launch",
        slideSub: "Local agent · shippable",
        slideBody: "One sentence starts Skills that land ",
        slideStrong: "slides you can present.",
        slideSteps: ["Cover", "Arch", "KPI", "Act"],
        steps: [
          "Send the prompt in Workbench; optionally use DeepSeek Flash",
          "Agent runs anycode-ppt from editorial templates",
          "run slides/ validates and builds index.html",
          "Present in the browser with ← →",
        ],
      },
      courseware: {
        tag: "HTML PPT",
        title: "Market-research courseware",
        summary:
          "County coffee case: 5-step method, catchment KPIs, anti-patterns, checklist. 8-page FDE editorial.",
        prompt:
          "Build a county-coffee market-research training deck: cover, agenda, 5-step method, catchment KPIs, survey-stack vs decision sample, insight, checklist, wrap-up. FDE editorial via anycode-ppt.",
        output: "8-page HTML deck + index viewer",
        slideTitle: "Method",
        slideSub: "Five steps",
        slideBody: "Question → sample → collect → cross-check →",
        slideStrong: "decide.",
        slideSteps: ["Cover", "Method", "Data", "Act"],
        steps: [
          "State audience and decision (open / relocate / no-go)",
          "Copy anycode-ppt cover / ladder / metrics / duo / checklist templates",
          "run slides/ to validate and build index.html",
          "Present in the browser with ← →",
        ],
      },
      "weekly-report": {
        tag: "DOCX",
        title: "Weekly / formal report",
        summary: "Structured Word: context, progress, risks, next steps.",
        prompt: "Write a weekly report: context, progress, risks, next steps — formal tone.",
        output: "Editable .docx",
        slideTitle: "Weekly",
        slideSub: "Structured prose",
        slideBody: "Clear headings, paragraphs ready to ",
        slideStrong: "submit for review.",
        slideSteps: ["Ctx", "Prog", "Risk", "Next"],
        steps: [
          "Give date range and project context",
          "Agent runs anycode-docx",
          "Open the Word file locally",
          "Adjust numbers and send",
        ],
      },
      "ops-sheet": {
        tag: "XLSX",
        title: "Ops spreadsheet",
        summary: "KPI strip + workbook chrome, auditable cross-sheet formulas, margin trend at a glance.",
        prompt: "Build an ops workbook: revenue, cost, margin — formulas and cross-sheet totals.",
        output: "Auditable .xlsx",
        slideTitle: "Ops sheet",
        slideSub: "Auditable formulas",
        slideBody: "KPIs and linked sheets with numbers you can ",
        slideStrong: "verify on the spot.",
        slideSteps: ["KPI", "Rev", "Cost", "Sum"],
        steps: [
          "Specify metrics and time grain",
          "Agent runs anycode-xlsx",
          "Formulas and checks are written in",
          "Open in Excel / Numbers to audit",
        ],
      },
    },
  },
} as const;

export type CaseItemId = keyof (typeof COPY)["zh"]["items"];

export function caseCopy(locale: CaseLocale) {
  return COPY[locale];
}

export function getCase(id: string): CaseDef | undefined {
  return CASES.find((c) => c.id === id);
}

export function casePath(id: string): string {
  return `/cases/${id}`;
}

export function featuredCase(): CaseDef {
  return CASES.find((c) => c.featured) ?? CASES[0];
}

export function gridCases(): CaseDef[] {
  return CASES.filter((c) => !c.featured);
}
