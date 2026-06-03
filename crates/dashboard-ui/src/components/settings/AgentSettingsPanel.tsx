import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import type { AgentProfileRecord } from "@/api/types/agents";
import { AgentEditorDrawer } from "@/components/settings/AgentEditorDrawer";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function AgentSettingsPanel() {
  const t = useT();
  const qc = useQueryClient();
  const profiles = useQuery({
    queryKey: ["agent-profiles"],
    queryFn: () => api.agentProfiles(),
  });
  const [editId, setEditId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const del = useMutation({
    mutationFn: (id: string) => api.deleteAgentProfile(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["agent-profiles"] }),
  });

  const rows = profiles.data?.profiles ?? [];
  const custom = rows.filter((p) => !p.builtin);
  const builtins = rows.filter((p) => p.builtin);

  return (
    <>
      <SectionCard
        title={t("settings.agents.builtinTitle")}
      >
        <p className="text-sm text-secondary m-0 mb-3">{t("settings.agents.builtinHint")}</p>
        <div className="overflow-x-auto">
          <table className="dw-table">
            <thead>
              <tr>
                <th>{t("common.id")}</th>
                <th>{t("settings.agents.extends")}</th>
                <th>{t("common.name")}</th>
              </tr>
            </thead>
            <tbody>
              {builtins.map((p) => (
                <ProfileRow key={p.id} profile={p} readonly />
              ))}
            </tbody>
          </table>
        </div>
      </SectionCard>

      <SectionCard
        title={t("settings.agents.customTitle")}
        action={
          <button
            type="button"
            className="dw-btn-secondary"
            onClick={() => {
              setEditId("");
              setCreating(true);
            }}
          >
            <Icon name="add" size={16} />
            {t("settings.agents.create")}
          </button>
        }
      >
        {custom.length === 0 ? (
          <p className="text-sm text-secondary m-0">{t("settings.agents.emptyCustom")}</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="dw-table">
              <thead>
                <tr>
                  <th>{t("common.id")}</th>
                  <th>{t("settings.agents.extends")}</th>
                  <th>{t("common.name")}</th>
                  <th className="text-right">{t("common.actions")}</th>
                </tr>
              </thead>
              <tbody>
                {custom.map((p) => (
                  <ProfileRow
                    key={p.id}
                    profile={p}
                    onEdit={() => {
                      setCreating(false);
                      setEditId(p.id);
                    }}
                    onDelete={() => {
                      if (window.confirm(t("settings.agents.deleteConfirm").replace("{id}", p.id))) {
                        del.mutate(p.id);
                      }
                    }}
                  />
                ))}
              </tbody>
            </table>
          </div>
        )}
      </SectionCard>

      {(creating || editId !== null) && (
        <AgentEditorDrawer
          profileId={creating ? undefined : (editId ?? undefined)}
          onClose={() => {
            setEditId(null);
            setCreating(false);
          }}
          onSaved={() => {
            setEditId(null);
            setCreating(false);
            qc.invalidateQueries({ queryKey: ["agent-profiles"] });
          }}
        />
      )}
    </>
  );
}

function ProfileRow({
  profile,
  readonly,
  onEdit,
  onDelete,
}: {
  profile: AgentProfileRecord;
  readonly?: boolean;
  onEdit?: () => void;
  onDelete?: () => void;
}) {
  const t = useT();
  return (
    <tr>
      <td>
        <code className="font-code text-xs">{profile.id}</code>
      </td>
      <td className="font-code text-xs text-secondary">{profile.extends}</td>
      <td>{profile.description || "—"}</td>
      <td className="text-right">
        {!readonly && (
          <div className="flex justify-end gap-2">
            <button type="button" className="dw-btn-ghost text-xs" onClick={onEdit}>
              {t("common.details")}
            </button>
            <button type="button" className="dw-btn-ghost text-xs text-error" onClick={onDelete}>
              {t("common.delete")}
            </button>
          </div>
        )}
      </td>
    </tr>
  );
}
