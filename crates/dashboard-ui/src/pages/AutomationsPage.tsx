import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { CronJobRecord } from "@/api/types";
import { Icon } from "@/components/Icon";
import { CcPageShell } from "@/components/ui/CcPageShell";
import { ListPageToolbar } from "@/components/ui/ListPageToolbar";
import { PageHeader } from "@/components/ui/PageHeader";
import { useControlCenter } from "@/context/ControlCenterContext";
import { useT, useLocale } from "@/i18n/context";
import type { EmbeddedPageProps } from "@/lib/pageProps";
import {
  buildCreateViaChatPrompt,
  buildCreateViaChatPromptFromTemplate,
  buildEditViaChatPrompt,
} from "@/lib/automationChatPrompts";
import {
  normalizeTemplate,
  templateText,
  jobDisplayName,
  type AutomationTemplate,
} from "@/lib/automationTemplates";
import { setComposerSeed } from "@/lib/composerSeed";
import { formatNextRunLine } from "@/lib/cronDisplay";
import { AutomationsOpsPanel } from "@/components/AutomationsOpsPanel";

type FilterTab = "all" | "enabled" | "paused";

export function AutomationsPage(_props: EmbeddedPageProps = {}) {
  const t = useT();
  const locale = useLocale();
  const navigate = useNavigate();
  const { closeControlCenter } = useControlCenter();
  const queryClient = useQueryClient();
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<FilterTab>("all");
  const [actionError, setActionError] = useState<string | null>(null);
  const [opsOpen, setOpsOpen] = useState(false);
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const cronJobs = useQuery({
    queryKey: ["cron-jobs"],
    queryFn: api.cronJobs,
    refetchInterval: 30_000,
  });
  const templates = useQuery({
    queryKey: ["cron-templates"],
    queryFn: api.cronTemplates,
  });

  const toggleJob = useMutation({
    mutationFn: (job: CronJobRecord) =>
      api.patchCronJob(job.id, { enabled: !(job.enabled ?? true) }),
    onMutate: () => setActionError(null),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["cron-jobs"] }),
    onError: (e: Error) => setActionError(e.message),
  });

  const deleteJob = useMutation({
    mutationFn: (jobId: string) => api.deleteCronJob(jobId),
    onMutate: () => setActionError(null),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["cron-jobs"] }),
    onError: (e: Error) => setActionError(e.message),
  });

  const templateList = useMemo(
    () => (templates.data?.templates ?? []).map((tpl) => normalizeTemplate(tpl)),
    [templates.data?.templates],
  );

  const existingTemplateIds = useMemo(() => {
    const jobs = cronJobs.data?.jobs ?? [];
    const names = new Set(jobs.map((j) => (j.name ?? "").trim().toLowerCase()).filter(Boolean));
    const ids = new Set<string>();
    for (const tpl of templateList) {
      const label = templateText(tpl.name, locale).toLowerCase();
      if (names.has(label)) ids.add(tpl.id);
    }
    return ids;
  }, [cronJobs.data?.jobs, templateList, locale]);

  const suggestionTemplates = templateList.filter((tpl) => !existingTemplateIds.has(tpl.id));

  const filteredJobs = useMemo(() => {
    const q = search.trim().toLowerCase();
    return (cronJobs.data?.jobs ?? []).filter((job) => {
      const enabled = job.enabled ?? true;
      if (filter === "enabled" && !enabled) return false;
      if (filter === "paused" && enabled) return false;
      if (!q) return true;
      const hay = [job.name, job.command, job.schedule, job.id]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      return hay.includes(q);
    });
  }, [cronJobs.data?.jobs, filter, search]);

  const goChatWithSeed = (text: string) => {
    setComposerSeed(text);
    closeControlCenter();
    void navigate({ to: "/" });
  };

  const openCreate = (template?: AutomationTemplate) => {
    const prompt = template
      ? buildCreateViaChatPromptFromTemplate(t, template, locale)
      : buildCreateViaChatPrompt(t);
    goChatWithSeed(prompt);
  };

  const openEdit = (job: CronJobRecord) => {
    goChatWithSeed(buildEditViaChatPrompt(t, job));
  };

  const filterToolbar = (
    <ListPageToolbar
      left={
        <>
          <div className="relative flex-1 min-w-0">
            <Icon
              name="search"
              size={16}
              className="absolute left-3 top-1/2 -translate-y-1/2 text-outline pointer-events-none"
            />
            <input
              className="dw-input dw-input--pill w-full pl-9"
              placeholder={t("automations.searchPlaceholder")}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              aria-label={t("automations.searchPlaceholder")}
            />
          </div>
          <div className="flex flex-wrap gap-2" role="tablist" aria-label={t("automations.filterTabs")}>
            {(["all", "enabled", "paused"] as const).map((tab) => (
              <button
                key={tab}
                type="button"
                role="tab"
                aria-selected={filter === tab}
                className={`dw-scheduled-tab ${filter === tab ? "dw-scheduled-tab--active" : ""}`}
                onClick={() => setFilter(tab)}
              >
                {t(`automations.filter.${tab}`)}
              </button>
            ))}
          </div>
        </>
      }
      actions={
        <button
          type="button"
          className="dw-btn-secondary dw-btn--pill text-sm"
          onClick={() => openCreate()}
        >
          <Icon name="add" size={16} />
          {t("automations.createBtn")}
        </button>
      }
    />
  );

  return (
    <CcPageShell
      className="dw-scheduled-tasks"
      header={
        <PageHeader
          title={t("automations.scheduledTitle")}
          subtitle={t("automations.scheduledSubtitle")}
          breadcrumbs={[
            { label: t("nav.home"), to: "/" },
            { label: t("automations.scheduledTitle") },
          ]}
        />
      }
    >
      <div className="max-w-3xl mx-auto w-full">
        <div className="dw-section-card dw-list-card mb-8">
          <div className="dw-list-card__toolbar px-3 py-3 border-b border-outline-variant/40">
            {filterToolbar}
          </div>
          <div className="dw-list-card__scroll px-1">
            {actionError && (
              <p className="text-xs text-error m-0 px-3 pt-3" role="alert">
                {actionError}
              </p>
            )}
            {cronJobs.isLoading ? (
              <p className="text-sm text-secondary m-0 px-3 py-4">{t("common.loading")}</p>
            ) : filteredJobs.length === 0 ? (
              <p className="text-sm text-secondary m-0 py-6 px-3 text-center">
                {search || filter !== "all"
                  ? t("automations.noMatchingJobs")
                  : t("automations.noScheduledJobs")}
              </p>
            ) : (
              <ul className="list-none m-0 p-0 flex flex-col">
                {filteredJobs.map((job) => (
                  <ScheduledTaskRow
                    key={job.id}
                    job={job}
                    locale={locale}
                    expanded={expandedId === job.id}
                    toggling={toggleJob.isPending}
                    deleting={deleteJob.isPending}
                    onToggleExpand={() =>
                      setExpandedId((id) => (id === job.id ? null : job.id))
                    }
                    onToggle={() => toggleJob.mutate(job)}
                    onEdit={() => openEdit(job)}
                    onDelete={() => {
                      if (window.confirm(t("automations.deleteJobConfirm"))) {
                        deleteJob.mutate(job.id);
                      }
                    }}
                  />
                ))}
              </ul>
            )}
          </div>
        </div>

        {suggestionTemplates.length > 0 && (
          <section>
            <h2 className="text-base font-semibold m-0 mb-3 text-on-surface">
              {t("automations.suggestions")}
            </h2>
            <ul className="list-none m-0 p-0 flex flex-col gap-1">
              {suggestionTemplates.map((tpl) => (
                <li key={tpl.id}>
                  <button
                    type="button"
                    className="dw-template-card w-full text-left"
                    onClick={() => openCreate(tpl)}
                  >
                    <span className="dw-template-card__icon" aria-hidden>
                      <Icon name={tpl.icon ?? "schedule"} size={18} />
                    </span>
                    <span className="min-w-0 flex-1">
                      <span className="dw-template-card__title">
                        {templateText(tpl.name, locale)}
                        <span className="dw-template-card__schedule">
                          {templateText(tpl.schedule_label, locale, tpl.schedule)}
                        </span>
                      </span>
                      {tpl.description && (
                        <span className="dw-template-card__desc">
                          {templateText(tpl.description, locale)}
                        </span>
                      )}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </section>
        )}

        <div className="mt-8 pt-4 border-t border-outline-variant">
          <button
            type="button"
            className="inline-flex items-center gap-1 border-0 bg-transparent p-0 text-xs text-secondary hover:text-primary cursor-pointer"
            onClick={() => setOpsOpen((v) => !v)}
            aria-expanded={opsOpen}
          >
            <Icon name={opsOpen ? "expand_less" : "expand_more"} size={16} />
            {opsOpen ? t("automations.hideOps") : t("automations.showOps")}
          </button>
          {opsOpen && <AutomationsOpsPanel className="mt-4" />}
        </div>
      </div>
    </CcPageShell>
  );
}

function ScheduledTaskRow({
  job,
  locale,
  expanded,
  toggling,
  deleting,
  onToggleExpand,
  onToggle,
  onEdit,
  onDelete,
}: {
  job: CronJobRecord;
  locale: "en" | "zh";
  expanded: boolean;
  toggling: boolean;
  deleting: boolean;
  onToggleExpand: () => void;
  onToggle: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const t = useT();
  const enabled = job.enabled ?? true;
  const title = jobDisplayName(job);
  const meta = formatNextRunLine(job.schedule, job.next_run_at, locale);
  const commandPreview =
    job.command.length > 120 ? `${job.command.slice(0, 117)}…` : job.command;

  return (
    <li className="dw-scheduled-row group" style={{ flexDirection: "column", alignItems: "stretch" }}>
      <div className="flex w-full items-center gap-2">
        <button
          type="button"
          className={`dw-scheduled-row__status ${enabled ? "" : "dw-scheduled-row__status--paused"}`}
          onClick={onToggle}
          disabled={toggling}
          aria-label={enabled ? t("automations.pauseJob") : t("automations.resumeJob")}
          title={enabled ? t("automations.pauseJob") : t("automations.resumeJob")}
        />
        <button
          type="button"
          className="min-w-0 flex-1 text-left border-0 bg-transparent p-0 cursor-pointer"
          onClick={onToggleExpand}
          aria-expanded={expanded}
          aria-label={expanded ? t("automations.collapseJob") : t("automations.expandJob")}
        >
          <p className="m-0 font-medium text-on-surface truncate">{title}</p>
          <p className="m-0 text-xs text-secondary mt-0.5 truncate">{meta}</p>
          {!expanded && (
            <p className="m-0 text-xs text-secondary/80 mt-0.5 truncate font-code">
              {commandPreview}
            </p>
          )}
        </button>
        <div className="dw-scheduled-row__actions shrink-0 flex items-center gap-0.5">
          <button
            type="button"
            className="dw-btn-ghost p-1"
            onClick={onEdit}
            aria-label={t("automations.editViaChat")}
            title={t("automations.editViaChat")}
          >
            <Icon name="edit" size={16} />
          </button>
          <button
            type="button"
            className="dw-btn-ghost p-1 text-error"
            disabled={deleting}
            onClick={onDelete}
            aria-label={t("automations.deleteJob")}
            title={t("automations.deleteJob")}
          >
            <Icon name="delete" size={16} />
          </button>
        </div>
      </div>
      {expanded && (
        <dl className="m-0 mt-2 ml-7 mr-2 mb-2 grid gap-1.5 text-xs text-secondary">
          <div className="flex gap-2">
            <dt className="shrink-0 font-medium text-on-surface/70">{t("automations.jobSchedule")}</dt>
            <dd className="m-0 font-code break-all">{job.schedule}</dd>
          </div>
          <div className="flex gap-2">
            <dt className="shrink-0 font-medium text-on-surface/70">{t("automations.jobNextRun")}</dt>
            <dd className="m-0">{meta}</dd>
          </div>
          <div className="flex gap-2">
            <dt className="shrink-0 font-medium text-on-surface/70">{t("automations.jobTimezone")}</dt>
            <dd className="m-0">{job.schedule_timezone ?? "local"}</dd>
          </div>
          <div className="flex gap-2">
            <dt className="shrink-0 font-medium text-on-surface/70">{t("automations.jobCommand")}</dt>
            <dd className="m-0 whitespace-pre-wrap break-words">{job.command}</dd>
          </div>
          <div className="flex gap-2">
            <dt className="shrink-0 font-medium text-on-surface/70">
              {t("automations.jobFailureDest")}
            </dt>
            <dd className="m-0">{job.failure_destination ?? "log"}</dd>
          </div>
          <div className="flex gap-2">
            <dt className="shrink-0 font-medium text-on-surface/70">
              {t("automations.jobToolProfile")}
            </dt>
            <dd className="m-0">{job.tool_profile ?? "default"}</dd>
          </div>
          <div className="flex gap-2">
            <dt className="shrink-0 font-medium text-on-surface/70">ID</dt>
            <dd className="m-0 font-code break-all">{job.id}</dd>
          </div>
        </dl>
      )}
    </li>
  );
}
