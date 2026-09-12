import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { HarnessGraphWorkbench } from "@/features/harness/HarnessGraphWorkbench";
import type { Graph } from "@/features/harness/graph-model.mjs";
import { PageHeader } from "@/components/ui/PageHeader";
import {
  desktopPairingOriginConfigured,
  desktopPairingPost,
  isTauriDesktop,
  persistHarnessDeviceToken,
  setHarnessSsoBearer,
} from "@/lib/desktopShell";
import { openExternal } from "@/lib/openExternal";

const SAMPLE: Graph = {
  version: 1,
  name: "human-pause",
  nodes: [
    {
      id: "ask",
      kind: { type: "human", question: "Approve this harness graph run?" },
    },
  ],
};

export function HarnessGraphPage() {
  const queryClient = useQueryClient();
  const [runId, setRunId] = useState<string | null>(null);
  const status = useQuery({
    queryKey: ["harness-status"],
    queryFn: () => api.harnessStatus(),
  });
  const projects = useQuery({
    queryKey: ["projects"],
    queryFn: () => api.projects(),
  });
  const projectId = projects.data?.projects?.[0]?.id;
  const run = useQuery({
    queryKey: ["harness-graph", projectId, runId],
    enabled: Boolean(projectId && status.data?.graph && runId),
    queryFn: () => api.harnessGraph(projectId!, runId!),
  });
  const start = useMutation({
    mutationFn: (graph: Graph) => api.harnessGraphStart(projectId!, graph),
    onSuccess: async (result) => {
      if (result.run_id) setRunId(result.run_id);
      await queryClient.invalidateQueries({ queryKey: ["harness-graph"] });
    },
  });
  const resume = useMutation({
    mutationFn: (id: string) => api.harnessGraphResume(projectId!, id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["harness-graph"] }),
  });
  const resolve = useMutation({
    mutationFn: (input: {
      runId: string;
      revision: number;
      nodeId: string;
      approved: boolean;
    }) =>
      api.harnessGraphResolve(projectId!, input.runId, {
        revision: input.revision,
        node_id: input.nodeId,
        approved: input.approved,
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["harness-graph"] }),
  });
  const [pairingNote, setPairingNote] = useState("");
  const pairDesktop = useMutation({
    mutationFn: async () => {
      const remote = await desktopPairingOriginConfigured();
      if (remote) {
        const begin = await desktopPairingPost<{
          authorize_url: string;
          poll_token: string;
        }>("/api/harness/pairing/sso/begin", {});
        try {
          await openExternal(begin.authorize_url);
        } catch {
          /* poll still works after a system-browser login */
        }
        let access: string | null = null;
        for (let i = 0; i < 60; i += 1) {
          const hop = await desktopPairingPost<{
            access_token?: string;
            pending?: boolean;
          }>("/api/harness/pairing/sso/poll", { poll_token: begin.poll_token });
          if (hop.access_token) {
            access = hop.access_token;
            break;
          }
          await new Promise((resolve) => setTimeout(resolve, 1000));
        }
        if (!access) {
          throw new Error("818cloud SSO hop timed out");
        }
        setHarnessSsoBearer(access);
      }
      const challenge = remote
        ? await desktopPairingPost<{ challenge: string; device_id: string }>(
            "/api/harness/pairing/challenges",
            { label: "desktop" },
          )
        : await api.harnessPairingChallenge("desktop");
      const confirmed = remote
        ? await desktopPairingPost<{
            device_id: string;
            token: string;
            keychain_service: string;
            keychain_account: string;
          }>("/api/harness/pairing/confirm", { challenge: challenge.challenge })
        : await api.harnessPairingConfirm(challenge.challenge);
      if (
        !confirmed.keychain_service ||
        !confirmed.keychain_account ||
        !confirmed.token
      ) {
        throw new Error(
          ("error" in confirmed && typeof confirmed.error === "string"
            ? confirmed.error
            : "pairing confirm missing keychain fields"),
        );
      }
      const stored = await persistHarnessDeviceToken(
        confirmed.keychain_service,
        confirmed.keychain_account,
        confirmed.token,
      );
      if (!stored.ok) {
        throw new Error(stored.error);
      }
      return confirmed.device_id;
    },
    onSuccess: (deviceId) => {
      setPairingNote(`Paired device ${deviceId}. Token is in the OS keychain only.`);
    },
    onError: (err) => {
      const message =
        err instanceof Error
          ? err.message
          : typeof err === "string"
            ? err
            : JSON.stringify(err);
      setPairingNote(message || "pairing failed");
    },
  });
  const armed = Boolean(status.data?.graph && projectId);
  const callbacks = useMemo(() => {
    if (!armed) return {};
    return {
      onStart: async (graph: Graph) => {
        await start.mutateAsync(graph);
      },
      onResume: async (id: string) => {
        await resume.mutateAsync(id);
      },
      onResolve: async (id: string, revision: number, nodeId: string, approved: boolean) => {
        await resolve.mutateAsync({ runId: id, revision, nodeId, approved });
      },
    };
  }, [armed, resume, resolve, start]);
  return (
    <div className="p-6">
      <PageHeader
        title="Harness Graph"
        subtitle={
          armed
            ? "Start and resume call the product GraphRunner. Work nodes stay fail-closed without a sandboxed Kernel host."
            : "Preview only. Arm harness-v1-graph and open a project before Start/Resume enable."
        }
      />
      {isTauriDesktop() && (
        <p className="mb-3 text-sm">
          <button
            type="button"
            disabled={pairDesktop.isPending}
            onClick={() => pairDesktop.mutate()}
          >
            Pair this desktop
          </button>
          {status.data?.pairing === "accounts_sso"
            ? " 818cloud pairing BFF is on this server."
            : " Local device pairing. Desktop outbound pairing uses host ANYCODE_HARNESS_PAIRING_ORIGIN only."}
          {pairingNote ? ` ${pairingNote}` : ""}
        </p>
      )}
      <HarnessGraphWorkbench
        graph={SAMPLE}
        checkpoint={run.data?.checkpoint ?? null}
        {...callbacks}
      />
    </div>
  );
}
