/** Built-in catalog when GET /api/project-templates is missing or fails (e.g. old dashboard). */
export type ProjectTemplateOption = {
  id: string;
  name: string;
  name_zh?: string;
  description: string;
  description_zh?: string;
  default_dir: string;
};

export const FALLBACK_PROJECT_TEMPLATES: ProjectTemplateOption[] = [
  {
    id: "flutter-app",
    name: "Flutter App",
    name_zh: "Flutter 应用",
    description:
      "Agent-first Flutter MVP: scaffold at create, bootstrap Flutter SDK when needed, then gates.",
    description_zh:
      "Agent 自主 Flutter：创建时仅骨架，由 Agent 安装 SDK 与平台目录，再跑门禁与 Goal。",
    default_dir: "my_flutter_app",
  },
];

export function resolveProjectTemplates(
  fromApi: ProjectTemplateOption[] | undefined,
  apiFailed: boolean,
): { templates: ProjectTemplateOption[]; usedFallback: boolean } {
  if (fromApi && fromApi.length > 0) {
    return { templates: fromApi, usedFallback: false };
  }
  return { templates: FALLBACK_PROJECT_TEMPLATES, usedFallback: apiFailed || !fromApi?.length };
}
