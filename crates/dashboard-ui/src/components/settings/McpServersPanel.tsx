import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

const EXAMPLE = `[
  {
    "slug": "filesystem",
    "command": "npx -y @modelcontextprotocol/server-filesystem /tmp"
  },
  {
    "slug": "api",
    "type": "http",
    "url": "https://example.com/mcp"
  }
]`;

export function McpServersPanel() {
  const t = useT();
  const queryClient = useQueryClient();
  const serversQuery = useQuery({
    queryKey: ["mcp-servers"],
    queryFn: api.mcpServers,
  });
  const [draft, setDraft] = useState<string | null>(null);
  const [parseError, setParseError] = useState<string | null>(null);

  const save = useMutation({
    mutationFn: (servers: unknown[]) => api.setMcpServers(servers),
    onSuccess: () => {
      setParseError(null);
      void queryClient.invalidateQueries({ queryKey: ["mcp-servers"] });
      void queryClient.invalidateQueries({ queryKey: ["doctor"] });
    },
  });

  const servers = serversQuery.data?.servers ?? [];
  const text =
    draft ??
    (servers.length > 0 ? JSON.stringify(servers, null, 2) : EXAMPLE);

  return (
    <SectionCard title={t("settings.mcpServers.title")}>
      <p className="text-sm text-secondary m-0 mb-3">{t("settings.mcpServers.hint")}</p>
      <textarea
        className="dw-input font-code text-xs min-h-[10rem] w-full"
        value={text}
        onChange={(e) => {
          setDraft(e.target.value);
          setParseError(null);
        }}
        spellCheck={false}
      />
      {parseError && <p className="text-sm text-error m-0 mt-2">{parseError}</p>}
      {save.isError && (
        <p className="text-sm text-error m-0 mt-2">{t("settings.mcpServers.error")}</p>
      )}
      <div className="flex flex-wrap items-center gap-2 mt-3">
        <button
          type="button"
          className="dw-btn-primary inline-flex items-center gap-2"
          disabled={save.isPending || serversQuery.isLoading}
          onClick={() => {
            try {
              const parsed = JSON.parse(text);
              if (!Array.isArray(parsed)) {
                setParseError(t("settings.mcpServers.invalidArray"));
                return;
              }
              save.mutate(parsed);
              setDraft(null);
            } catch {
              setParseError(t("settings.mcpServers.invalidJson"));
            }
          }}
        >
          <Icon name="save" size={16} />
          {t("settings.mcpServers.save")}
        </button>
        {save.data?.restart_hint && (
          <span className="text-xs text-secondary">{save.data.restart_hint}</span>
        )}
      </div>
    </SectionCard>
  );
}
