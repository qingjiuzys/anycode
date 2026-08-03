import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

type Props = {
  projectId: string;
};

export function ConversationGitBar({ projectId }: Props) {
  const t = useT();
  const queryClient = useQueryClient();
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  const statusQuery = useQuery({
    queryKey: ["project-git-status", projectId],
    queryFn: () => api.projectGitStatus(projectId),
    refetchInterval: 8_000,
    staleTime: 4_000,
  });

  const git = statusQuery.data?.git;

  useEffect(() => {
    if (!menuOpen) return;
    const onDoc = (e: MouseEvent) => {
      if (menuRef.current?.contains(e.target as Node)) return;
      setMenuOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [menuOpen]);

  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ["project-git-status", projectId] });
  };

  const commit = useMutation({
    mutationFn: (message?: string) => api.projectGitCommit(projectId, { message }),
    onSuccess: invalidate,
  });

  const push = useMutation({
    mutationFn: () => api.projectGitPush(projectId),
    onSuccess: invalidate,
  });

  const busy = commit.isPending || push.isPending;

  if (!git?.is_repo) return null;

  const branchLabel = git.branch ?? t("git.detached");
  const syncHint =
    git.has_upstream && (git.ahead > 0 || git.behind > 0)
      ? t("git.syncHint")
          .replace("{ahead}", String(git.ahead))
          .replace("{behind}", String(git.behind))
      : null;

  const runCommit = () => {
    setMenuOpen(false);
    commit.mutate(undefined);
  };

  const runCommitPush = async () => {
    setMenuOpen(false);
    try {
      await commit.mutateAsync(undefined);
      await push.mutateAsync();
    } catch {
      /* errors surface via mutation state */
    }
  };

  const runPush = () => {
    setMenuOpen(false);
    push.mutate();
  };

  const actionError = (commit.error ?? push.error) as Error | null;

  return (
    <div className="conv-git-bar px-1 pb-2">
      <div className="conv-git-bar__row">
        <div className="conv-git-bar__pill conv-git-bar__pill--changes" title={branchLabel}>
          <span className="conv-git-bar__label">{t("git.changes")}</span>
          <span className="conv-git-bar__stat conv-git-bar__stat--add">+{git.insertions}</span>
          <span className="conv-git-bar__stat conv-git-bar__stat--del">-{git.deletions}</span>
          {syncHint ? (
            <span className="conv-git-bar__sync text-[11px] text-secondary ml-1">{syncHint}</span>
          ) : null}
        </div>

        <div className="relative" ref={menuRef}>
          <button
            type="button"
            className="conv-git-bar__pill conv-git-bar__pill--action"
            disabled={busy || (!git.has_changes && git.ahead === 0)}
            onClick={() => setMenuOpen((v) => !v)}
            aria-expanded={menuOpen}
            aria-haspopup="menu"
          >
            <span>{t("git.commitAndPush")}</span>
            <Icon name="expand_more" size={16} />
          </button>
          {menuOpen ? (
            <div className="conv-git-bar__menu" role="menu">
              <button
                type="button"
                role="menuitem"
                className="conv-git-bar__menu-item"
                disabled={busy || !git.has_changes}
                onClick={runCommit}
              >
                {t("git.commit")}
              </button>
              <button
                type="button"
                role="menuitem"
                className="conv-git-bar__menu-item"
                disabled={busy || (!git.has_changes && git.ahead === 0)}
                onClick={() => void runCommitPush()}
              >
                {t("git.commitAndPush")}
              </button>
              <button
                type="button"
                role="menuitem"
                className="conv-git-bar__menu-item"
                disabled={busy || !git.has_upstream || (git.ahead === 0 && !git.has_changes)}
                onClick={runPush}
              >
                {t("git.push")}
              </button>
            </div>
          ) : null}
        </div>
      </div>
      {actionError ? (
        <p className="text-xs text-error m-0 mt-1 px-1">{actionError.message}</p>
      ) : null}
    </div>
  );
}
