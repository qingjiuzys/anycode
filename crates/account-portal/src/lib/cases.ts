export type CaseKind = "ppt" | "doc" | "sheet";

export type CaseLocale = "zh" | "en";

export type CaseDef = {
  id: string;
  kind: CaseKind;
  skill: string;
  model: string;
  featured?: boolean;
};

export const CASES: CaseDef[] = [
  {
    id: "launch-ppt",
    kind: "ppt",
    skill: "anycode-ppt",
    model: "DeepSeek Flash",
    featured: true,
  },
  {
    id: "courseware",
    kind: "ppt",
    skill: "anycode-ppt",
    model: "DeepSeek Flash",
  },
  {
    id: "weekly-report",
    kind: "doc",
    skill: "anycode-docx",
    model: "DeepSeek Flash",
  },
  {
    id: "ops-sheet",
    kind: "sheet",
    skill: "anycode-xlsx",
    model: "DeepSeek Flash",
  },
];

const COPY = {
  zh: {
    sectionKicker: "CASES",
    sectionTitle: "交付案例",
    sectionLead: "一句话启动，本机落盘可验收文件。",
    viewCase: "查看案例",
    backHome: "返回首页",
    promptLabel: "提示词",
    modelLabel: "模型",
    skillLabel: "Skill",
    outputLabel: "产物",
    stepsLabel: "怎么跑通",
    tryLabel: "自己试一次",
    tryBody: "下载 anyCode，用同一句话在本机生成。",
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
        title: "培训课件翻页稿",
        summary: "封面 → 机制 → 案例 → 小结，一页一命题。",
        prompt: "做一份培训课件：封面、机制、案例、小结，每页只讲一个命题。",
        output: "分页 HTML 课件",
        slideTitle: "机制",
        slideSub: "一页一命题",
        slideBody: "先讲清因果，再给",
        slideStrong: "可复述的结论。",
        slideSteps: ["封面", "机制", "案例", "小结"],
        steps: [
          "描述受众与课时目标",
          "选用 ladder / section / quote 等模板页",
          "校验密度与视觉契约",
          "交付 slides/ + index.html",
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
        summary: "多 sheet、公式与校验，直接可复核。",
        prompt: "做一份经营数据表：收入、成本、毛利，含公式与跨 sheet 汇总。",
        output: "可复核 .xlsx",
        slideTitle: "经营表",
        slideSub: "公式可审计",
        slideBody: "多 sheet 联动，数字",
        slideStrong: "可当场复核。",
        slideSteps: ["收入", "成本", "毛利", "汇总"],
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
    backHome: "Back home",
    promptLabel: "Prompt",
    modelLabel: "Model",
    skillLabel: "Skill",
    outputLabel: "Output",
    stepsLabel: "How it runs",
    tryLabel: "Try it yourself",
    tryBody: "Download anyCode and run the same prompt locally.",
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
        title: "Training courseware",
        summary: "Cover → mechanism → cases → wrap-up. One thesis per slide.",
        prompt: "Make a training deck: cover, mechanism, cases, wrap-up — one thesis per slide.",
        output: "Paginated HTML courseware",
        slideTitle: "Mechanism",
        slideSub: "One thesis / slide",
        slideBody: "Cause first, then a ",
        slideStrong: "retellable conclusion.",
        slideSteps: ["Cover", "Mech", "Case", "End"],
        steps: [
          "Describe audience and lesson goal",
          "Pick ladder / section / quote templates",
          "Validate density and visual contract",
          "Ship slides/ + index.html",
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
        summary: "Multi-sheet workbook with auditable formulas.",
        prompt: "Build an ops workbook: revenue, cost, margin — formulas and cross-sheet totals.",
        output: "Auditable .xlsx",
        slideTitle: "Ops sheet",
        slideSub: "Auditable formulas",
        slideBody: "Linked sheets with numbers you can ",
        slideStrong: "verify on the spot.",
        slideSteps: ["Rev", "Cost", "Margin", "Sum"],
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
