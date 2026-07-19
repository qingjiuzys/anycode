import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { api } from "@/api/client";
import type { LocalModelStatus } from "@/api/client/core";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";
import { isTauriDesktop } from "@/lib/desktopShell";

const queryKey = ["local-models"];

export function LocalRuntimePanel() {
  const t = useT();
  const queryClient = useQueryClient();
  const models = useQuery({
    queryKey,
    queryFn: () => api.localModels(),
    retry: isTauriDesktop() ? 3 : 1,
    refetchInterval: (query) =>
      query.state.data?.models?.some(
        (model) => model.phase === "downloading" || model.phase === "starting",
      )
        ? 1500
        : false,
  });
  const refresh = () => void queryClient.invalidateQueries({ queryKey });
  const action = useMutation({
    mutationFn: ({ id, operation }: { id: string; operation: string }) => {
      switch (operation) {
        case "download":
          return api.localModelDownload(id);
        case "cancel":
          return api.localModelCancelDownload(id);
        case "start":
          return api.localModelStart(id);
        case "stop":
          return api.localModelStop(id);
        case "delete":
          return api.localModelDelete(id);
        default:
          throw new Error(`unknown local runtime operation: ${operation}`);
      }
    },
    onSuccess: refresh,
  });

  if (!isTauriDesktop()) return null;

  return (
    <SectionCard title={t("settings.model.localRuntime.title")}>
      <p className="text-sm text-secondary m-0 mb-3">
        {t("settings.model.localRuntime.hint")}
      </p>
      {models.isLoading ? (
        <p className="text-sm text-secondary m-0">{t("settings.model.localRuntime.loading")}</p>
      ) : null}
      {models.isError ? (
        <div className="rounded-lg border border-error/40 bg-error/5 p-3 mb-3" role="alert">
          <p className="text-sm text-error m-0">{t("settings.model.localRuntime.loadError")}</p>
          <p className="text-xs text-secondary m-0 mt-1 break-all">
            {models.error instanceof Error ? models.error.message : String(models.error)}
          </p>
          <button
            type="button"
            className="dw-btn-secondary text-xs mt-2"
            onClick={() => void models.refetch()}
          >
            {t("settings.model.localRuntime.retry")}
          </button>
        </div>
      ) : null}
      <div className="space-y-3">
        {models.data?.models.map((model) => (
          <RuntimeCard
            key={model.id}
            model={model}
            pending={action.isPending && action.variables?.id === model.id}
            run={(operation) => action.mutate({ id: model.id, operation })}
          />
        ))}
        {!models.isLoading && !models.isError && (models.data?.models.length ?? 0) === 0 ? (
          <p className="text-sm text-secondary m-0">{t("settings.model.localRuntime.empty")}</p>
        ) : null}
      </div>
    </SectionCard>
  );
}

function RuntimeCard({
  model,
  pending,
  run,
}: {
  model: LocalModelStatus;
  pending: boolean;
  run: (operation: string) => void;
}) {
  const t = useT();
  const percent =
    model.download_total > 0
      ? Math.min(100, Math.round((model.download_bytes / model.download_total) * 100))
      : 0;
  return (
    <article className="rounded-lg border border-outline-variant/40 p-3">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold m-0">
            {model.display_name}
            {model.preview ? (
              <span className="ml-2 text-xs font-normal text-secondary">Preview</span>
            ) : null}
          </h3>
          <p className="text-xs text-secondary m-0 mt-1">
            {model.id} · {model.version}
          </p>
        </div>
        <span className="text-xs">{model.phase}</span>
      </div>
      <dl className="grid grid-cols-2 md:grid-cols-4 gap-2 text-xs my-3">
        <div>
          <dt className="text-secondary">{t("settings.model.localRuntime.context")}</dt>
          <dd className="m-0">{model.context_tokens.toLocaleString()}</dd>
        </div>
        <div>
          <dt className="text-secondary">{t("settings.model.localRuntime.architecture")}</dt>
          <dd className="m-0">{model.architectures.join(", ")}</dd>
        </div>
        <div>
          <dt className="text-secondary">{t("settings.model.localRuntime.license")}</dt>
          <dd className="m-0">{model.license}</dd>
        </div>
        <div>
          <dt className="text-secondary">{t("settings.model.localRuntime.size")}</dt>
          <dd className="m-0">{(model.size_bytes / 1_000_000).toFixed(0)} MB</dd>
        </div>
      </dl>
      {model.last_error ? (
        <p className="text-xs text-error m-0 mb-2" role="alert">
          {model.last_error}
        </p>
      ) : null}
      {model.phase === "downloading" ? (
        <div className="mb-3">
          <div className="h-2 rounded-full bg-surface-container-high overflow-hidden">
            <div
              className="h-full bg-primary transition-[width] duration-150"
              style={{ width: `${percent}%` }}
            />
          </div>
          <p className="text-xs text-secondary m-0 mt-1">{percent}%</p>
        </div>
      ) : null}
      <div className="flex flex-wrap gap-2">
        {(model.phase === "not_installed" || model.phase === "error") && (
          <ActionButton disabled={pending} onClick={() => run("download")}>
            {t("settings.model.localRuntime.download")}
          </ActionButton>
        )}
        {model.phase === "downloading" && (
          <ActionButton disabled={pending} onClick={() => run("cancel")}>
            {t("settings.model.localRuntime.cancel")}
          </ActionButton>
        )}
        {(model.phase === "ready" || (model.phase === "error" && model.model_path)) && (
          <ActionButton disabled={pending} onClick={() => run("start")}>
            {t("settings.model.localRuntime.start")}
          </ActionButton>
        )}
        {model.phase === "running" && (
          <ActionButton disabled={pending} onClick={() => run("stop")}>
            {t("settings.model.localRuntime.stop")}
          </ActionButton>
        )}
        {model.model_path && model.phase !== "running" && (
          <ActionButton disabled={pending} onClick={() => run("delete")}>
            {t("settings.model.localRuntime.delete")}
          </ActionButton>
        )}
      </div>
      {model.preview && !model.capabilities.tools ? (
        <p className="text-xs text-secondary m-0 mt-3">
          {t("settings.model.localRuntime.previewNote")}
        </p>
      ) : null}
    </article>
  );
}

function ActionButton({
  children,
  disabled,
  onClick,
}: {
  children: ReactNode;
  disabled: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className="dw-btn-secondary text-sm"
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}
