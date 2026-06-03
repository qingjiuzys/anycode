import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { ModelProfile, ModelsConfig, SpeechModelsConfig } from "@/api/types";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

type CapabilityTab = "embedding" | "speech" | "image" | "video";

const TABS: CapabilityTab[] = ["embedding", "speech", "image", "video"];

const TEST_CAPABILITY: Record<CapabilityTab, string> = {
  embedding: "embedding",
  speech: "stt",
  image: "image",
  video: "video",
};

function emptyProfile(): ModelProfile {
  return { provider: "", model: "", api_key: "" };
}

function profileFrom(source?: ModelProfile | null): ModelProfile {
  return {
    provider: source?.provider ?? "",
    model: source?.model ?? "",
    api_key: "",
    base_url: source?.base_url ?? "",
    plan: source?.plan ?? "",
  };
}

function profileToPatch(source: ModelProfile): ModelProfile | undefined {
  const provider = source.provider?.trim();
  const model = source.model?.trim();
  const apiKey = source.api_key?.trim();
  const baseUrl = source.base_url?.trim();
  const plan = source.plan?.trim();
  if (!provider && !model && !apiKey && !baseUrl && !plan) return undefined;
  return {
    provider: provider || undefined,
    model: model || undefined,
    api_key: apiKey || undefined,
    base_url: baseUrl || undefined,
    plan: plan || undefined,
  };
}

function ProfileFields({
  profile,
  onChange,
  providers,
  idPrefix,
}: {
  profile: ModelProfile;
  onChange: (next: ModelProfile) => void;
  providers: { id: string; label: string }[];
  idPrefix: string;
}) {
  const t = useT();
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
      <label className="flex flex-col gap-1 text-sm">
        <span className="text-secondary font-medium">{t("settings.model.provider")}</span>
        <select
          className="dw-input font-code"
          value={profile.provider ?? ""}
          onChange={(e) => onChange({ ...profile, provider: e.target.value })}
        >
          <option value="">{t("settings.model.inheritGlobal")}</option>
          {providers.map((p) => (
            <option key={p.id} value={p.id}>
              {p.label}
            </option>
          ))}
        </select>
      </label>
      <label className="flex flex-col gap-1 text-sm">
        <span className="text-secondary font-medium">{t("settings.model.model")}</span>
        <input
          className="dw-input font-code"
          value={profile.model ?? ""}
          onChange={(e) => onChange({ ...profile, model: e.target.value })}
        />
      </label>
      <label className="flex flex-col gap-1 text-sm sm:col-span-2">
        <span className="text-secondary font-medium">{t("settings.model.apiKey")}</span>
        <input
          type="password"
          className="dw-input font-code"
          value={profile.api_key ?? ""}
          onChange={(e) => onChange({ ...profile, api_key: e.target.value })}
          placeholder={t("settings.model.apiKeyOptional")}
          autoComplete="off"
        />
      </label>
      <label className="flex flex-col gap-1 text-sm sm:col-span-2" htmlFor={`${idPrefix}-base-url`}>
        <span className="text-secondary font-medium">{t("settings.model.baseUrl")}</span>
        <input
          id={`${idPrefix}-base-url`}
          className="dw-input font-code"
          value={profile.base_url ?? ""}
          onChange={(e) => onChange({ ...profile, base_url: e.target.value })}
        />
      </label>
    </div>
  );
}

export function ModelCapabilityTabs() {
  const t = useT();
  const qc = useQueryClient();
  const [tab, setTab] = useState<CapabilityTab>("embedding");
  const [embedding, setEmbedding] = useState<ModelProfile>(emptyProfile());
  const [speechStt, setSpeechStt] = useState<ModelProfile>(emptyProfile());
  const [speechTts, setSpeechTts] = useState<ModelProfile>(emptyProfile());
  const [image, setImage] = useState<ModelProfile>(emptyProfile());
  const [video, setVideo] = useState<ModelProfile>(emptyProfile());

  const catalog = useQuery({
    queryKey: ["model-catalog"],
    queryFn: () => api.modelCatalog(),
  });

  const llm = useQuery({
    queryKey: ["llm-config"],
    queryFn: () => api.getLlmConfig(),
  });

  useEffect(() => {
    const models = llm.data?.models;
    if (!models) return;
    setEmbedding(profileFrom(models.embedding));
    setSpeechStt(profileFrom(models.speech?.stt));
    setSpeechTts(profileFrom(models.speech?.tts));
    setImage(profileFrom(models.image));
    setVideo(profileFrom(models.video));
  }, [llm.data?.models]);

  const buildModelsPatch = (): ModelsConfig => {
    const speech: SpeechModelsConfig = {};
    const stt = profileToPatch(speechStt);
    const tts = profileToPatch(speechTts);
    if (stt) speech.stt = stt;
    if (tts) speech.tts = tts;
    return {
      embedding: profileToPatch(embedding),
      speech: stt || tts ? speech : undefined,
      image: profileToPatch(image),
      video: profileToPatch(video),
    };
  };

  const save = useMutation({
    mutationFn: () => api.putLlmConfig({ models: buildModelsPatch() }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["llm-config"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
    },
  });

  const test = useMutation({
    mutationFn: (capability: string) => api.testLlm(capability),
  });

  const providers = catalog.data?.providers ?? [];

  return (
    <SectionCard title={t("settings.model.capabilitiesTitle")}>
      <div className="flex flex-wrap gap-2 mb-4">
        {TABS.map((id) => (
          <button
            key={id}
            type="button"
            className={`dw-chip${tab === id ? " active" : ""}`}
            onClick={() => setTab(id)}
          >
            {t(`settings.model.capabilities.${id}`)}
          </button>
        ))}
      </div>

      {tab === "embedding" && (
        <ProfileFields
          profile={embedding}
          onChange={setEmbedding}
          providers={providers}
          idPrefix="embedding"
        />
      )}

      {tab === "speech" && (
        <div className="space-y-6">
          <div>
            <h4 className="text-sm font-medium m-0 mb-3">{t("settings.model.capabilities.stt")}</h4>
            <ProfileFields
              profile={speechStt}
              onChange={setSpeechStt}
              providers={providers}
              idPrefix="speech-stt"
            />
          </div>
          <div>
            <h4 className="text-sm font-medium m-0 mb-3">{t("settings.model.capabilities.tts")}</h4>
            <ProfileFields
              profile={speechTts}
              onChange={setSpeechTts}
              providers={providers}
              idPrefix="speech-tts"
            />
          </div>
        </div>
      )}

      {tab === "image" && (
        <ProfileFields profile={image} onChange={setImage} providers={providers} idPrefix="image" />
      )}

      {tab === "video" && (
        <ProfileFields profile={video} onChange={setVideo} providers={providers} idPrefix="video" />
      )}

      <div className="flex flex-wrap items-center gap-2 mt-4 pt-4 border-t border-outline-variant">
        <button
          type="button"
          className="dw-btn-primary"
          disabled={save.isPending || llm.isLoading}
          onClick={() => save.mutate()}
        >
          {save.isPending ? t("common.loading") : t("settings.model.saveCapabilities")}
        </button>
        <button
          type="button"
          className="dw-btn-secondary"
          disabled={test.isPending}
          onClick={() => test.mutate(TEST_CAPABILITY[tab])}
        >
          {test.isPending ? t("common.loading") : t("settings.model.testCapability")}
        </button>
        {tab === "speech" && (
          <button
            type="button"
            className="dw-btn-secondary"
            disabled={test.isPending}
            onClick={() => test.mutate("tts")}
          >
            {test.isPending ? t("common.loading") : t("settings.model.testTts")}
          </button>
        )}
      </div>

      {save.isSuccess && (
        <p className="text-sm text-secondary mt-2 m-0">{t("settings.model.saved")}</p>
      )}
      {save.isError && (
        <div className="dw-alert-error mt-2">{(save.error as Error).message}</div>
      )}
      {test.isSuccess && test.data?.ok && (
        <p className="text-sm text-secondary mt-2 m-0">
          {test.data.message ?? t("settings.model.testSuccess")}
        </p>
      )}
      {test.isError && (
        <div className="dw-alert-error mt-2">{(test.error as Error).message}</div>
      )}
      {test.isSuccess && test.data && !test.data.ok && (
        <div className="dw-alert-error mt-2">{test.data.error ?? t("settings.model.testFailed")}</div>
      )}
    </SectionCard>
  );
}
