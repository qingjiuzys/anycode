import { useMutation, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

const EXTENDS_OPTIONS = [
  "general-purpose",
  "explore",
  "plan",
  "workspace-assistant",
  "goal",
];

type Props = {
  profileId?: string;
  onClose: () => void;
  onSaved: () => void;
};

export function AgentEditorDrawer({ profileId, onClose, onSaved }: Props) {
  const t = useT();
  const isNew = !profileId;
  const existing = useQuery({
    queryKey: ["agent-profile", profileId],
    queryFn: () => api.agentProfile(profileId!),
    enabled: !!profileId,
  });

  const [id, setId] = useState(profileId ?? "");
  const [extendsId, setExtendsId] = useState("general-purpose");
  const [description, setDescription] = useState("");
  const [toolsDeny, setToolsDeny] = useState("");
  const [skillsAllowlist, setSkillsAllowlist] = useState("");

  useEffect(() => {
    const p = existing.data?.profile;
    if (!p) return;
    setExtendsId(p.extends || "general-purpose");
    setDescription(p.description);
    try {
      const tools = JSON.parse(p.tools_json) as { deny?: string[]; allow?: string[] };
      setToolsDeny((tools.deny ?? []).join(", "));
    } catch {
      setToolsDeny("");
    }
    try {
      const skills = JSON.parse(p.skills_json) as { allowlist?: string[] };
      setSkillsAllowlist((skills.allowlist ?? []).join(", "));
    } catch {
      setSkillsAllowlist("");
    }
  }, [existing.data?.profile]);

  const save = useMutation({
    mutationFn: () => {
      const targetId = (isNew ? id : profileId!).trim();
      const deny = toolsDeny
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
      const allowlist = skillsAllowlist
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
      return api.putAgentProfile(targetId, {
        extends: extendsId,
        description: description.trim() || undefined,
        tools_json: deny.length > 0 ? { deny } : undefined,
        skills_json: allowlist.length > 0 ? { allowlist } : undefined,
      });
    },
    onSuccess: () => onSaved(),
  });

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-scrim/40">
      <div className="w-full max-w-lg h-full bg-surface shadow-xl flex flex-col">
        <div className="flex items-center justify-between p-4 border-b border-outline-variant">
          <h2 className="text-lg font-semibold m-0">
            {isNew ? t("settings.agents.create") : t("settings.agents.edit")}
          </h2>
          <button type="button" className="dw-btn-ghost" onClick={onClose} aria-label={t("common.back")}>
            <Icon name="close" size={20} />
          </button>
        </div>
        <div className="p-4 space-y-4 overflow-y-auto flex-1">
          {isNew && (
            <div>
              <label className="block text-xs font-medium text-secondary mb-1">{t("common.id")}</label>
              <input
                className="dw-input w-full font-code"
                value={id}
                onChange={(e) => setId(e.target.value)}
                placeholder="reviewer"
              />
            </div>
          )}
          <div>
            <label className="block text-xs font-medium text-secondary mb-1">
              {t("settings.agents.extends")}
            </label>
            <select
              className="dw-input w-full"
              value={extendsId}
              onChange={(e) => setExtendsId(e.target.value)}
            >
              {EXTENDS_OPTIONS.map((opt) => (
                <option key={opt} value={opt}>
                  {opt}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-xs font-medium text-secondary mb-1">{t("common.name")}</label>
            <input
              className="dw-input w-full"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-secondary mb-1">
              {t("settings.agents.toolsDeny")}
            </label>
            <input
              className="dw-input w-full font-code text-xs"
              value={toolsDeny}
              onChange={(e) => setToolsDeny(e.target.value)}
              placeholder="Bash, Edit, FileWrite"
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-secondary mb-1">
              {t("settings.agents.skillsAllowlist")}
            </label>
            <input
              className="dw-input w-full font-code text-xs"
              value={skillsAllowlist}
              onChange={(e) => setSkillsAllowlist(e.target.value)}
              placeholder="skill-a, skill-b"
            />
          </div>
        </div>
        <div className="p-4 border-t border-outline-variant flex justify-end gap-2">
          <button type="button" className="dw-btn-secondary" onClick={onClose}>
            {t("common.back")}
          </button>
          <button
            type="button"
            className="dw-btn-primary"
            disabled={save.isPending || (isNew && !id.trim())}
            onClick={() => save.mutate()}
          >
            {save.isPending ? t("common.loading") : t("common.save")}
          </button>
        </div>
      </div>
    </div>
  );
}
