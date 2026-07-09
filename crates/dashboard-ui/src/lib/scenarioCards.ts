export type ScenarioCard = {
  id: string;
  icon: string;
  agent: string;
  skills: string[];
  promptEn: string;
  promptZh: string;
};

export const SCENARIO_CARDS: ScenarioCard[] = [
  {
    id: "coding",
    icon: "code",
    agent: "builder",
    skills: [],
    promptEn:
      "Help me build a small web feature end-to-end: clarify requirements, scaffold files, implement, and run checks.",
    promptZh: "帮我从零实现一个小功能：澄清需求、搭建文件结构、编码实现并运行检查。",
  },
  {
    id: "ppt",
    icon: "slideshow",
    agent: "office-writer",
    skills: ["office-pptx"],
    promptEn:
      "Create a 8–12 slide presentation deck (.pptx) for our product update. Outline first, then generate slides with clear titles and bullet points.",
    promptZh:
      "为我们的产品更新制作 8–12 页演示文稿（.pptx）：先列大纲，再生成标题与要点清晰的幻灯片。",
  },
  {
    id: "video",
    icon: "movie",
    agent: "general-purpose",
    skills: ["video-script"],
    promptEn:
      "Plan a 60–90s explainer video: write a shot-by-shot script, then generate key visual assets with image/video tools.",
    promptZh:
      "策划一条 60–90 秒讲解视频：写分镜脚本，并用图像/视频工具生成关键素材。",
  },
  {
    id: "novel",
    icon: "menu_book",
    agent: "office-writer",
    skills: ["novel-writer"],
    promptEn:
      "Start a short novel: propose a title and chapter outline (6 chapters), then write chapter 1 to chapters/01.md.",
    promptZh:
      "开始写一部短篇小说：给出书名与 6 章大纲，并把第 1 章写入 chapters/01.md。",
  },
];

export function scenarioPrompt(card: ScenarioCard, locale: "zh" | "en"): string {
  return locale === "zh" ? card.promptZh : card.promptEn;
}
