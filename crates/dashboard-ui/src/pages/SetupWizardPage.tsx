import { useCallback, useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import type { QuickAuthPreset } from "@/api/types/setup";
import { api } from "@/api/client";
import { DiscordChannelPanel } from "@/components/channels/DiscordChannelPanel";
import { TelegramChannelPanel } from "@/components/channels/TelegramChannelPanel";
import { BrandMark } from "@/components/BrandMark";
import { NewProjectDialog } from "@/components/NewProjectDialog";
import { useT } from "@/i18n/context";

const WIZARD_STEPS = [
  "welcome",
  "model",
  "test",
  "memory",
  "skills",
  "channels",
  "projects",
  "done",
] as const;

type WizardStep = (typeof WIZARD_STEPS)[number];

function stepIndex(step: WizardStep) {
  return WIZARD_STEPS.indexOf(step);
}

function providerNeedsPlan(provider: string) {
  const id = provider.trim().toLowerCase();
  return id === "z.ai" || id === "zai" || id === "bigmodel";
}

export function SetupWizardPage() {
  const t = useT();
  const navigate = useNavigate();
  const qc = useQueryClient();
  const search = useSearch({ strict: false }) as {
    review?: string;
    step?: string;
    tab?: string;
  };
  const reviewMode = search.review === "1";

  const initialStep = useMemo((): WizardStep => {
    const raw = search.step?.trim();
    if (raw && (WIZARD_STEPS as readonly string[]).includes(raw)) {
      return raw as WizardStep;
    }
    return "welcome";
  }, [search.step]);

  const [step, setStep] = useState<WizardStep>(initialStep);
  const [newProjectOpen, setNewProjectOpen] = useState(false);

  const status = useQuery({
    queryKey: ["setup-status"],
    queryFn: () => api.setupStatus(),
  });

  const ensureWorkspace = useMutation({
    mutationFn: () => api.setupEnsureWorkspace(),
  });

  useEffect(() => {
    if (step === "welcome" && !ensureWorkspace.isSuccess && !ensureWorkspace.isPending) {
      ensureWorkspace.mutate();
    }
  }, [step, ensureWorkspace]);

  const platform = status.data?.setup.platform ?? "unknown";
  const isWindows = platform === "windows";

  const goNext = useCallback(() => {
    const idx = stepIndex(step);
    if (idx < WIZARD_STEPS.length - 1) {
      setStep(WIZARD_STEPS[idx + 1]);
    }
  }, [step]);

  const goBack = useCallback(() => {
    const idx = stepIndex(step);
    if (idx > 0) {
      setStep(WIZARD_STEPS[idx - 1]);
    }
  }, [step]);

  const finish = useMutation({
    mutationFn: () => api.setupComplete({ scan_projects: true }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["setup-status"] });
      await navigate({ to: "/" });
    },
  });

  const stepLabels = useMemo(
    () =>
      WIZARD_STEPS.map((id) => ({
        id,
        label: t(`setup.steps.${id}`),
      })),
    [t],
  );

  if (status.isLoading) {
    return (
      <div className="setup-wizard setup-wizard--loading">
        <p className="text-secondary">{t("common.loading")}</p>
      </div>
    );
  }

  return (
    <div className="setup-wizard">
      <header className="setup-wizard__header">
        <BrandMark />
        <h1 className="setup-wizard__title">{t("setup.title")}</h1>
        {reviewMode && (
          <Link to="/settings" search={{ section: "model" }} className="setup-wizard__review-link">
            {t("setup.backToSettings")}
          </Link>
        )}
      </header>

      <nav className="setup-wizard__nav" aria-label={t("setup.progressAria")}>
        {stepLabels.map(({ id, label }, i) => (
          <span
            key={id}
            className={`setup-wizard__nav-item${stepIndex(step) >= i ? " setup-wizard__nav-item--active" : ""}${step === id ? " setup-wizard__nav-item--current" : ""}`}
          >
            {label}
          </span>
        ))}
      </nav>

      <main className="setup-wizard__main">
        {step === "welcome" && (
          <SetupWelcomeStep onNext={goNext} platform={platform} />
        )}
        {step === "model" && <SetupModelStep onNext={goNext} onBack={goBack} />}
        {step === "test" && <SetupTestStep onNext={goNext} onBack={goBack} />}
        {step === "memory" && <SetupMemoryStep onNext={goNext} onBack={goBack} />}
        {step === "skills" && <SetupSkillsStep onNext={goNext} onBack={goBack} />}
        {step === "channels" && (
          <SetupChannelsStep
            onNext={goNext}
            onBack={goBack}
            isWindows={isWindows}
            initialTab={search.tab}
          />
        )}
        {step === "projects" && (
          <SetupProjectsStep
            onNext={goNext}
            onBack={goBack}
            onNewProject={() => setNewProjectOpen(true)}
          />
        )}
        {step === "done" && (
          <SetupDoneStep
            onFinish={() => finish.mutate()}
            finishing={finish.isPending}
            error={finish.error instanceof Error ? finish.error.message : null}
          />
        )}
      </main>

      <NewProjectDialog open={newProjectOpen} onClose={() => setNewProjectOpen(false)} />
    </div>
  );
}

function SetupWelcomeStep({
  onNext,
  platform,
}: {
  onNext: () => void;
  platform: string;
}) {
  const t = useT();
  return (
    <section className="setup-step">
      <h2>{t("setup.welcome.title")}</h2>
      <p className="text-secondary">{t("setup.welcome.body")}</p>
      <ul className="setup-step__list text-secondary text-sm">
        <li>{t("setup.welcome.pointLocal")}</li>
        <li>{t("setup.welcome.pointKey")}</li>
        <li>{t("setup.welcome.pointPlatform").replace("{platform}", platform)}</li>
      </ul>
      <div className="setup-step__actions">
        <button type="button" className="dw-btn dw-btn-primary" onClick={onNext}>
          {t("setup.start")}
        </button>
      </div>
    </section>
  );
}

function SetupModelStep({ onNext, onBack }: { onNext: () => void; onBack: () => void }) {
  const t = useT();
  const qc = useQueryClient();
  const catalog = useQuery({ queryKey: ["model-catalog"], queryFn: () => api.modelCatalog() });
  const quickAuth = useQuery({ queryKey: ["setup-quick-auth"], queryFn: () => api.setupQuickAuth() });
  const llm = useQuery({ queryKey: ["llm-config"], queryFn: () => api.getLlmConfig() });

  const [provider, setProvider] = useState("");
  const [model, setModel] = useState("");
  const [plan, setPlan] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKey, setApiKey] = useState("");

  useEffect(() => {
    if (!llm.data) return;
    setProvider(llm.data.provider ?? "");
    setModel(llm.data.model ?? "");
    setPlan(llm.data.plan ?? "");
    setBaseUrl(llm.data.base_url ?? "");
  }, [llm.data]);

  const applyPreset = (preset: QuickAuthPreset) => {
    setProvider(preset.provider);
    setModel(preset.default_model);
    setPlan(preset.plan);
    setBaseUrl(preset.base_url);
  };

  const save = useMutation({
    mutationFn: () =>
      api.putLlmConfig({
        provider: provider.trim() || undefined,
        model: model.trim() || undefined,
        plan: plan.trim() || undefined,
        base_url: baseUrl.trim() || undefined,
        api_key: apiKey.trim() || undefined,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["llm-config"] });
      qc.invalidateQueries({ queryKey: ["setup-status"] });
      onNext();
    },
  });

  const showPlan = providerNeedsPlan(provider);

  return (
    <section className="setup-step">
      <h2>{t("setup.model.title")}</h2>
      <p className="text-secondary text-sm mb-4">{t("setup.model.hint")}</p>

      {(quickAuth.data?.presets ?? []).length > 0 && (
        <div className="setup-quick-auth mb-4">
          <p className="text-sm font-medium mb-2">{t("setup.model.quickPresets")}</p>
          <div className="flex flex-wrap gap-2">
            {quickAuth.data!.presets.map((p) => (
              <button
                key={p.id}
                type="button"
                className="dw-btn dw-btn-secondary text-sm"
                onClick={() => applyPreset(p)}
              >
                {p.label}
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-4">
        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary">{t("settings.model.provider")}</span>
          <select
            className="dw-input font-code"
            value={provider}
            onChange={(e) => setProvider(e.target.value)}
          >
            <option value="">{t("settings.model.providerPlaceholder")}</option>
            {(catalog.data?.providers ?? []).map((p) => (
              <option key={p.id} value={p.id}>
                {p.label}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary">{t("settings.model.model")}</span>
          <input
            className="dw-input font-code"
            value={model}
            onChange={(e) => setModel(e.target.value)}
            placeholder={t("settings.model.modelFreeformPlaceholder")}
          />
        </label>
        {showPlan && (
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-secondary">{t("settings.model.plan")}</span>
            <input className="dw-input font-code" value={plan} onChange={(e) => setPlan(e.target.value)} />
          </label>
        )}
        <label className="flex flex-col gap-1 text-sm sm:col-span-2">
          <span className="text-secondary">{t("settings.model.baseUrl")}</span>
          <input
            className="dw-input font-code"
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
            placeholder="https://"
          />
        </label>
        <label className="flex flex-col gap-1 text-sm sm:col-span-2">
          <span className="text-secondary">{t("settings.model.apiKey")}</span>
          <input
            className="dw-input font-code"
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder={t("setup.model.apiKeyPlaceholder")}
            autoComplete="off"
          />
        </label>
      </div>

      {save.isError && (
        <div className="dw-alert-error mb-3">{(save.error as Error).message}</div>
      )}

      <div className="setup-step__actions">
        <button type="button" className="dw-btn dw-btn-secondary" onClick={onBack}>
          {t("common.back")}
        </button>
        <button
          type="button"
          className="dw-btn dw-btn-primary"
          disabled={save.isPending || !provider.trim() || !model.trim()}
          onClick={() => save.mutate()}
        >
          {save.isPending ? t("common.saving") : t("common.continue")}
        </button>
      </div>
    </section>
  );
}

function SetupTestStep({ onNext, onBack }: { onNext: () => void; onBack: () => void }) {
  const t = useT();
  const test = useMutation({ mutationFn: () => api.testLlm("chat") });

  return (
    <section className="setup-step">
      <h2>{t("setup.test.title")}</h2>
      <p className="text-secondary text-sm mb-4">{t("setup.test.hint")}</p>

      <button
        type="button"
        className="dw-btn dw-btn-secondary mb-3"
        disabled={test.isPending}
        onClick={() => test.mutate()}
      >
        {test.isPending ? t("setup.test.running") : t("setup.test.run")}
      </button>

      {test.isSuccess && test.data?.ok && (
        <div className="dw-alert-success mb-3">{t("setup.test.ok")}</div>
      )}
      {test.isSuccess && !test.data?.ok && (
        <div className="dw-alert-error mb-3">
          {test.data?.error ?? t("setup.test.fail")}
        </div>
      )}
      {test.isError && (
        <div className="dw-alert-error mb-3">{(test.error as Error).message}</div>
      )}

      <div className="setup-step__actions">
        <button type="button" className="dw-btn dw-btn-secondary" onClick={onBack}>
          {t("common.back")}
        </button>
        <button
          type="button"
          className="dw-btn dw-btn-primary"
          disabled={!test.isSuccess || !test.data?.ok}
          onClick={onNext}
        >
          {t("common.continue")}
        </button>
      </div>
    </section>
  );
}

function SetupMemoryStep({ onNext, onBack }: { onNext: () => void; onBack: () => void }) {
  const t = useT();
  const [preset, setPreset] = useState("hybrid");
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [embedUrl, setEmbedUrl] = useState("");
  const [embedModel, setEmbedModel] = useState("text-embedding-3-small");

  const save = useMutation({
    mutationFn: () =>
      api.setupMemory({
        preset: showAdvanced && preset === "pipeline_http" ? "pipeline_http" : preset,
        embedding_base_url: showAdvanced ? embedUrl : undefined,
        embedding_model: showAdvanced ? embedModel : undefined,
      }),
    onSuccess: () => onNext(),
  });

  const options = [
    { id: "noop", label: t("setup.memory.noop"), desc: t("setup.memory.noopDesc") },
    { id: "simple_file", label: t("setup.memory.simple"), desc: t("setup.memory.simpleDesc") },
    { id: "hybrid", label: t("setup.memory.hybrid"), desc: t("setup.memory.hybridDesc") },
    {
      id: "pipeline_no_embedding",
      label: t("setup.memory.pipelineNoEmbed"),
      desc: t("setup.memory.pipelineNoEmbedDesc"),
    },
  ];

  return (
    <section className="setup-step">
      <h2>{t("setup.memory.title")}</h2>
      <p className="text-secondary text-sm mb-4">{t("setup.memory.hint")}</p>

      <div className="flex flex-col gap-2 mb-4">
        {options.map((o) => (
          <label key={o.id} className="setup-memory-option">
            <input
              type="radio"
              name="memory"
              checked={preset === o.id && !showAdvanced}
              onChange={() => {
                setPreset(o.id);
                setShowAdvanced(false);
              }}
            />
            <span>
              <strong>{o.label}</strong>
              <span className="block text-secondary text-xs">{o.desc}</span>
            </span>
          </label>
        ))}
        <label className="setup-memory-option">
          <input
            type="radio"
            name="memory"
            checked={showAdvanced}
            onChange={() => {
              setShowAdvanced(true);
              setPreset("pipeline_http");
            }}
          />
          <span>
            <strong>{t("setup.memory.advanced")}</strong>
            <span className="block text-secondary text-xs">{t("setup.memory.advancedDesc")}</span>
          </span>
        </label>
      </div>

      {showAdvanced && (
        <div className="grid gap-2 mb-4">
          <input
            className="dw-input font-code"
            value={embedUrl}
            onChange={(e) => setEmbedUrl(e.target.value)}
            placeholder={t("setup.memory.embedUrl")}
          />
          <input
            className="dw-input font-code"
            value={embedModel}
            onChange={(e) => setEmbedModel(e.target.value)}
            placeholder={t("setup.memory.embedModel")}
          />
        </div>
      )}

      {save.isError && (
        <div className="dw-alert-error mb-3">{(save.error as Error).message}</div>
      )}

      <div className="setup-step__actions">
        <button type="button" className="dw-btn dw-btn-secondary" onClick={onBack}>
          {t("common.back")}
        </button>
        <button
          type="button"
          className="dw-btn dw-btn-primary"
          disabled={save.isPending}
          onClick={() => save.mutate()}
        >
          {save.isPending ? t("common.saving") : t("common.continue")}
        </button>
      </div>
    </section>
  );
}

function SetupSkillsStep({ onNext, onBack }: { onNext: () => void; onBack: () => void }) {
  const t = useT();
  const install = useMutation({ mutationFn: () => api.installStarterSkills() });

  return (
    <section className="setup-step">
      <h2>{t("setup.skills.title")}</h2>
      <p className="text-secondary text-sm mb-4">{t("setup.skills.hint")}</p>

      {install.isSuccess && (
        <div className="dw-alert-success mb-3">
          {t("agents.installStarterOk").replace("{count}", String(install.data?.count ?? 0))}
        </div>
      )}
      {install.isError && (
        <div className="dw-alert-error mb-3">{(install.error as Error).message}</div>
      )}

      <div className="setup-step__actions">
        <button type="button" className="dw-btn dw-btn-secondary" onClick={onBack}>
          {t("common.back")}
        </button>
        <button
          type="button"
          className="dw-btn dw-btn-secondary"
          disabled={install.isPending}
          onClick={() => install.mutate()}
        >
          {install.isPending ? t("common.loading") : t("agents.installStarterBtn")}
        </button>
        <button type="button" className="dw-btn dw-btn-primary" onClick={onNext}>
          {t("setup.skipContinue")}
        </button>
      </div>
    </section>
  );
}

type ChannelTab = "skip" | "telegram" | "discord" | "wechat";

function parseChannelTab(raw: string | undefined): ChannelTab {
  if (raw === "telegram" || raw === "discord" || raw === "wechat") {
    return raw;
  }
  return "skip";
}

function SetupChannelsStep({
  onNext,
  onBack,
  isWindows,
  initialTab,
}: {
  onNext: () => void;
  onBack: () => void;
  isWindows: boolean;
  initialTab?: string;
}) {
  const t = useT();
  const [tab, setTab] = useState<ChannelTab>(() => parseChannelTab(initialTab));

  const wechatQr = useQuery({
    queryKey: ["setup-wechat-qr"],
    queryFn: () => api.setupWechatQr(),
    enabled: tab === "wechat",
  });

  const qrcodeId = wechatQr.data?.qr?.qrcode_id ?? "";
  const wechatPoll = useQuery({
    queryKey: ["setup-wechat-poll", qrcodeId],
    queryFn: () => api.setupWechatStatus(qrcodeId),
    enabled: tab === "wechat" && qrcodeId.length > 0,
    refetchInterval: (q) =>
      q.state.data?.result?.status === "confirmed" ? false : 3000,
  });

  const wechatConfirmed = wechatPoll.data?.result?.status === "confirmed";

  return (
    <section className="setup-step">
      <h2>{t("setup.channels.title")}</h2>
      <p className="text-secondary text-sm mb-4">{t("setup.channels.hint")}</p>

      <div className="flex flex-wrap gap-2 mb-4">
        {(["skip", "telegram", "discord", "wechat"] as const).map((id) => (
          <button
            key={id}
            type="button"
            className={`dw-btn dw-btn-secondary text-sm${tab === id ? " dw-btn-primary" : ""}`}
            onClick={() => setTab(id)}
          >
            {t(`setup.channels.tab.${id}`)}
          </button>
        ))}
      </div>

      {tab === "telegram" && (
        <div className="mb-4">
          <TelegramChannelPanel />
        </div>
      )}

      {tab === "discord" && (
        <div className="mb-4">
          <DiscordChannelPanel />
        </div>
      )}

      {tab === "wechat" && (
        <div className="mb-4">
          {isWindows && (
            <p className="text-secondary text-sm mb-2">{t("setup.channels.wechatWindows")}</p>
          )}
          {!isWindows && (
            <p className="text-secondary text-sm mb-2">{t("setup.channels.wechatMac")}</p>
          )}
          {wechatQr.isLoading && <p className="text-secondary">{t("common.loading")}</p>}
          {wechatQr.isError && (
            <div className="dw-alert-error">{(wechatQr.error as Error).message}</div>
          )}
          {wechatQr.data?.qr?.content && (
            <div className="setup-wechat-qr">
              {wechatQr.data.qr.content.trimStart().startsWith("http") ? (
                <p className="font-code text-xs break-all">{wechatQr.data.qr.content}</p>
              ) : (
                <pre className="text-xs whitespace-pre-wrap font-code">
                  {wechatQr.data.qr.terminal_render ?? wechatQr.data.qr.content.slice(0, 200)}
                </pre>
              )}
              <p className="text-secondary text-sm mt-2">
                {wechatConfirmed
                  ? t("setup.channels.wechatOk")
                  : t("setup.channels.wechatScan")}
              </p>
            </div>
          )}
        </div>
      )}

      <div className="setup-step__actions">
        <button type="button" className="dw-btn dw-btn-secondary" onClick={onBack}>
          {t("common.back")}
        </button>
        <button type="button" className="dw-btn dw-btn-primary" onClick={onNext}>
          {t("setup.skipContinue")}
        </button>
      </div>
    </section>
  );
}

function SetupProjectsStep({
  onNext,
  onBack,
  onNewProject,
}: {
  onNext: () => void;
  onBack: () => void;
  onNewProject: () => void;
}) {
  const t = useT();
  const scan = useMutation({ mutationFn: () => api.scanProjects() });

  return (
    <section className="setup-step">
      <h2>{t("setup.projects.title")}</h2>
      <p className="text-secondary text-sm mb-4">{t("setup.projects.hint")}</p>

      {scan.isSuccess && (
        <div className="dw-alert-success mb-3">
          {t("setup.projects.scanOk").replace("{count}", String(scan.data?.projects_registered ?? 0))}
        </div>
      )}
      {scan.isError && (
        <div className="dw-alert-error mb-3">{(scan.error as Error).message}</div>
      )}

      <div className="flex flex-wrap gap-2 mb-4">
        <button
          type="button"
          className="dw-btn dw-btn-secondary"
          disabled={scan.isPending}
          onClick={() => scan.mutate()}
        >
          {scan.isPending ? t("common.loading") : t("projects.scanNew")}
        </button>
        <button type="button" className="dw-btn dw-btn-secondary" onClick={onNewProject}>
          {t("layout.newProject")}
        </button>
      </div>

      <div className="setup-step__actions">
        <button type="button" className="dw-btn dw-btn-secondary" onClick={onBack}>
          {t("common.back")}
        </button>
        <button type="button" className="dw-btn dw-btn-primary" onClick={onNext}>
          {t("common.continue")}
        </button>
      </div>
    </section>
  );
}

function SetupDoneStep({
  onFinish,
  finishing,
  error,
}: {
  onFinish: () => void;
  finishing: boolean;
  error: string | null;
}) {
  const t = useT();
  return (
    <section className="setup-step">
      <h2>{t("setup.done.title")}</h2>
      <p className="text-secondary text-sm mb-4">{t("setup.done.body")}</p>
      {error && <div className="dw-alert-error mb-3">{error}</div>}
      <div className="setup-step__actions">
        <button
          type="button"
          className="dw-btn dw-btn-primary"
          disabled={finishing}
          onClick={onFinish}
        >
          {finishing ? t("common.loading") : t("setup.done.start")}
        </button>
      </div>
    </section>
  );
}
