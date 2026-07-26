import type { BootstrapSummary } from "@/api/types";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function WorkspacePathsPanel({ bootstrap }: { bootstrap?: BootstrapSummary }) {
  const t = useT();
  const paths = bootstrap?.workspace_registered ?? [];

  return (
    <SectionCard title={t("home.workspacePaths")}>
      <p className="text-xs text-secondary m-0 mb-3">{t("projects.localHint")}</p>
      {paths.length === 0 ? (
        <p className="text-sm text-secondary m-0">{t("home.noProjects")}</p>
      ) : (
        <ul className="m-0 p-0 list-none space-y-2">
          {paths.map(([path, registered]) => (
            <li
              key={path}
              className="flex items-start gap-2 text-sm font-code text-secondary break-all"
            >
              <Icon
                name={registered ? "check_circle" : "radio_button_unchecked"}
                size={16}
                className={
                  registered ? "text-success shrink-0 mt-0.5" : "text-outline shrink-0 mt-0.5"
                }
              />
              <span>{path}</span>
              {!registered && (
                <span className="text-xs text-warn shrink-0">({t("home.notRegistered")})</span>
              )}
            </li>
          ))}
        </ul>
      )}
    </SectionCard>
  );
}
