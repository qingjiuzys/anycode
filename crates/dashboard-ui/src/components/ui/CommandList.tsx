import { CopyButton } from "./CopyButton";
import { useT } from "@/i18n/context";

export function CommandList({ commands }: { commands: string[] }) {
  const t = useT();
  const text = commands.join("\n");
  return (
    <div className="flex flex-col gap-3">
      <pre className="bg-surface-container-low border border-outline-variant rounded p-4 font-code text-xs overflow-auto max-h-48 whitespace-pre-wrap m-0">
        {text}
      </pre>
      <CopyButton text={text} label={t("common.copy")} />
    </div>
  );
}
