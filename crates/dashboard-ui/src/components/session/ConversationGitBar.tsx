import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";
import type { GitChangeKind, GitFileChange } from "@/api/types/workbench";

type Props = {
  projectId: string;
};

const KIND_BADGE: Record<GitChangeKind, { cls: string; key: string }> = {
  modified: { cls: "conv-git-bar__file-badge--modified", key: "git.modified" },
  added: { cls: "conv-git-bar__file-badge--added", key: "git.added" },
  deleted: { cls: "conv-git-bar__file-badge--deleted", key: "git.deleted" },
  renamed: { cls: "conv-git-bar__file-badge--renamed", key: "git.renamed" },
  untracked: { cls: "conv-git-bar__file-badge--untracked", key: "git.untracked" },
  type_changed: { cls: "conv-git-bar__file-badge--modified", key: "git.typeChanged" },
};

/** Render unified diff text with per-line colouring. */
function DiffBody({ diff }: { diff: string }) {
  const lines = useMemo(() => diff.split("\n"), [diff]);
  return (
    <pre className="conv-git-bar__diff-pre">
      {lines.map((line, i) => {
        let cls = "conv-diff-line";
        if (line.startsWith("+") && !line.startsWith("+++")) cls += " conv-diff-line--add";
        else if (line.startsWith("-") && !line.startsWith("---")) cls += " conv-diff-line--del";
        else if (line.startsWith("@@")) cls += " conv-diff-line--hunk";
        else if (line.startsWith("diff ") || line.startsWith("index ") || line.startsWith("---") || line.startsWith("+++") || line.startsWith("new file") || line.startsWith("deleted file")) cls += " conv-diff-line--meta";
        return (
          <span key={i} className={cls}>
            {line || " "}
          </span>
        );
      })}
    </pre>
  );
}

export function ConversationGitBar({ projectId }: Props) {
  const t = useT();
  const queryClient = useQueryClient();
  const [menuOpen, setMenuOpen] = useState(false);
  const [treeOpen, setTreeOpen] = useState(false);
  const [selected, setSelected] = useState<GitFileChange | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const statusQuery = useQuery({
    queryKey: ["project-git-status", projectId],
    queryFn: () => api.projectGitStatus(projectId),
    refetchInterval: 8_000,
    staleTime: 4_000,
  });

  const changesQuery = useQuery({
    queryKey: ["project-git-changes", projectId],
    queryFn: () => api.projectGitChanges(projectId),
    enabled: treeOpen || Boolean(selected),
    refetchInterval: 8_000,
    staleTime: 4_000,
  });

  const diffQuery = useQuery({
    queryKey: ["project-git-diff", projectId, selected?.path],
    queryFn: () =>
      api.projectGitFileDiff(projectId, selected!.path, selected!.kind),
    enabled: Boolean(selected),
  });

  const git = statusQuery.data?.git;
  const changes = changesQuery.data?.changes ?? [];

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
    void queryClient.invalidateQueries({ queryKey: ["project-git-changes", projectId] });
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
  const stagedCount = changes.filter((c) => c.staged).length;
  const unstagedCount = changes.length - stagedCount;

  const selectFile = (c: GitFileChange) => {
    setSelected(c);
  };

  return (
    <div className="conv-git-bar px-1 pb-2">
      <div className="conv-git-bar__row">
        <button
          type="button"
          className="conv-git-bar__pill conv-git-bar__pill--action"
          title={t("git.changedFiles")}
          onClick={() => {
            setTreeOpen((v) => !v);
            setSelected(null);
          }}
          aria-expanded={treeOpen}
        >
          <span className="conv-git-bar__label">{t("git.changes")}</span>
          <span className="conv-git-bar__stat conv-git-bar__stat--add">+{git.insertions}</span>
          <span className="conv-git-bar__stat conv-git-bar__stat--del">-{git.deletions}</span>
          {syncHint ? (
            <span className="conv-git-bar__sync text-[11px] text-secondary ml-1">{syncHint}</span>
          ) : null}
          <Icon name={treeOpen ? "expand_less" : "expand_more"} size={16} />
        </button>

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

      {selected ? (
        <div className="conv-git-bar__diff">
          <div className="conv-git-bar__diff-head">
            <button
              type="button"
              className="conv-git-bar__file-name inline-flex items-center gap-1 text-left border-0 bg-transparent text-xs text-secondary cursor-pointer hover:text-on-surface"
              onClick={() => setSelected(null)}
            >
              <Icon name="chevron_left" size={16} />
              {t("git.backToChanges")}
            </button>
            <span className="flex items-center gap-1 min-w-0 truncate">
              {(() => {
                const badge = KIND_BADGE[selected.kind];
                return (
                  <span className={`conv-git-bar__file-badge ${badge.cls}`}>{t(badge.key)}</span>
                );
              })()}
              <span className="text-xs font-mono text-on-surface truncate">{selected.path}</span>
            </span>
          </div>
          <div className="px-3 py-1 flex items-center gap-2 text-[11px] border-b border-outline-variant/50">
            <span className="text-success">+{diffQuery.data?.diff.insertions ?? 0}</span>
            <span className="text-error">-{diffQuery.data?.diff.deletions ?? 0}</span>
            {diffQuery.isLoading ? <span className="text-secondary">{t("common.loading")}</span> : null}
          </div>
          {diffQuery.data ? <DiffBody diff={diffQuery.data.diff.diff} /> : null}
          {diffQuery.isError ? (
            <p className="text-xs text-error m-0 px-3 py-2">
              {(diffQuery.error as Error).message}
            </p>
          ) : null}
        </div>
      ) : null}

      {treeOpen && !selected ? (
        <div className="conv-git-bar__tree">
          <div className="conv-git-bar__tree-head">
            <span className="inline-flex items-center gap-1 min-w-0">
              <Icon name="timeline" size={13} />
              <span className="truncate font-mono text-[10px] text-secondary">{branchLabel}</span>
            </span>
            <span className="shrink-0">
              {t("git.changedFiles")} ({changes.length})
            </span>
            {stagedCount > 0 ? (
              <span className="text-[10px]">
                {t("git.staged")} {stagedCount} · {t("git.unstaged")} {unstagedCount}
              </span>
            ) : null}
          </div>
          {changes.length === 0 ? (
            <p className="text-xs text-secondary text-center py-4 m-0">{t("git.noChanges")}</p>
          ) : (
            <div className="max-h-[18rem] overflow-auto py-1">
              {changes.map((c) => {
                const badge = KIND_BADGE[c.kind];
                return (
                  <button
                    type="button"
                    key={c.path}
                    className="conv-git-bar__file"
                    onClick={() => selectFile(c)}
                  >
                    <span className={`conv-git-bar__file-badge ${badge.cls}`}>{t(badge.key)}</span>
                    <span className="conv-git-bar__file-name">{c.path}</span>
                    <span className="text-[10px] tabular-nums shrink-0">
                      <span className="text-success">+{c.insertions}</span>{" "}
                      <span className="text-error">-{c.deletions}</span>
                    </span>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      ) : null}
    </div>
  );
}