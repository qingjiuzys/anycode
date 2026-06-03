import type {
  AgentProfileEffective,
  AgentProfileRecord,
  AgentProfileUpsertBody,
} from "@/api/types/agents";
import { del, get, put } from "../http";

export const agentsClient = {
  agentProfiles: () => get<{ profiles: AgentProfileRecord[] }>("/api/agents/profiles"),
  agentProfile: (id: string) =>
    get<{ profile: AgentProfileRecord }>(`/api/agents/profiles/${encodeURIComponent(id)}`),
  agentProfileEffective: (id: string) =>
    get<AgentProfileEffective>(`/api/agents/profiles/${encodeURIComponent(id)}/effective`),
  putAgentProfile: (id: string, body: AgentProfileUpsertBody) =>
    put<{ profile: AgentProfileRecord }>(`/api/agents/profiles/${encodeURIComponent(id)}`, body),
  deleteAgentProfile: (id: string) =>
    del<{ ok: boolean }>(`/api/agents/profiles/${encodeURIComponent(id)}`),
};
