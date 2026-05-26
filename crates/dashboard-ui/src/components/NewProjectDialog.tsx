import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

export function NewProjectDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const t = useT();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [rootPath, setRootPath] = useState("");
  const [name, setName] = useState("");
  const [createRoot, setCreateRoot] = useState(true);

  const create = useMutation({
    mutationFn: () =>
      api.upsertProject({
        root_path: rootPath.trim(),
        name: name.trim() || undefined,
        create_root: createRoot,
      }),
    onSuccess: (data) => {
      void queryClient.invalidateQueries({ queryKey: ["projects"] });
      onClose();
      setRootPath("");
      setName("");
      void navigate({
        to: "/projects/$projectId",
        params: { projectId: data.project.id },
      });
    },
  });

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[100] flex items-center justify-center bg-on-surface/25 p-4"
      role="dialog"
      aria-modal
      aria-labelledby="new-project-title"
    >
      <div className="w-full max-w-md bg-surface-container-lowest border border-outline-variant rounded-lg shadow-lg p-6">
        <div className="flex items-start justify-between gap-4 mb-4">
          <div>
            <h2 id="new-project-title" className="text-lg font-semibold m-0">
              {t("newProject.title")}
            </h2>
            <p className="text-sm text-secondary m-0 mt-1">{t("newProject.subtitle")}</p>
          </div>
          <button type="button" className="dw-btn-ghost p-1" onClick={onClose} aria-label={t("newProject.cancel")}>
            <Icon name="close" size={20} />
          </button>
        </div>
        <form
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            if (rootPath.trim()) create.mutate();
          }}
        >
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-secondary font-medium">{t("newProject.rootPath")}</span>
            <input
              className="dw-input font-code"
              value={rootPath}
              onChange={(e) => setRootPath(e.target.value)}
              placeholder={t("newProject.rootPathPlaceholder")}
              required
              autoFocus
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-secondary font-medium">{t("newProject.name")}</span>
            <input
              className="dw-input"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </label>
          <label className="flex items-center gap-2 text-sm text-secondary">
            <input
              type="checkbox"
              checked={createRoot}
              onChange={(e) => setCreateRoot(e.target.checked)}
            />
            {t("newProject.createRoot")}
          </label>
          {create.isError && (
            <div className="dw-alert-error">{(create.error as Error).message}</div>
          )}
          <div className="flex justify-end gap-2 pt-2">
            <button type="button" className="dw-btn-secondary" onClick={onClose}>
              {t("newProject.cancel")}
            </button>
            <button
              type="submit"
              className="dw-btn-primary"
              disabled={create.isPending || !rootPath.trim()}
            >
              {create.isPending ? t("common.loading") : t("newProject.submit")}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
