import { useQuery } from "@tanstack/react-query";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";
import { useMediaStatus } from "@/hooks/useMediaStatus";
import {
  getAppleMediaCapabilities,
  isTauriDesktop,
  type AppleMediaCapabilities,
} from "@/lib/desktopShell";

function CapRow({ label, ok }: { label: string; ok: boolean }) {
  return (
    <div className="flex items-center justify-between gap-3 text-sm py-1.5">
      <span className="text-secondary">{label}</span>
      <span className={ok ? "text-success font-medium" : "text-secondary"}>
        {ok ? "✓" : "—"}
      </span>
    </div>
  );
}

function CapabilitiesBlock({ caps, t }: { caps: AppleMediaCapabilities; t: (k: string) => string }) {
  return (
    <div className="border border-outline-variant rounded-lg p-3 flex flex-col gap-1">
      <CapRow label={t("settings.appleMedia.capStt")} ok={caps.stt} />
      <CapRow label={t("settings.appleMedia.capOcr")} ok={caps.ocr} />
      <CapRow label={t("settings.appleMedia.capTts")} ok={caps.tts} />
      <CapRow label={t("settings.appleMedia.capNotify")} ok={caps.notify} />
      <CapRow label={t("settings.appleMedia.capKeychain")} ok={caps.keychain} />
      <CapRow label={t("settings.appleMedia.capPasteboard")} ok={caps.pasteboard} />
      {caps.speech_authorized != null && (
        <CapRow
          label={t("settings.appleMedia.speechPermission")}
          ok={caps.speech_authorized === true}
        />
      )}
      {caps.microphone_authorized != null && (
        <CapRow
          label={t("settings.appleMedia.micPermission")}
          ok={caps.microphone_authorized === true}
        />
      )}
      {caps.helper_path ? (
        <p className="text-[11px] font-code text-secondary m-0 mt-2 truncate" title={caps.helper_path}>
          {caps.helper_path}
        </p>
      ) : null}
    </div>
  );
}

export function AppleMediaSettingsPanel() {
  const t = useT();
  const { appleMedia: serverCaps } = useMediaStatus();
  const desktopQuery = useQuery({
    queryKey: ["apple-media-capabilities"],
    queryFn: () => getAppleMediaCapabilities(),
    enabled: isTauriDesktop(),
    staleTime: 30_000,
  });

  const caps = desktopQuery.data ?? serverCaps ?? null;
  const isMac =
    caps?.platform === "macos" ||
    (typeof navigator !== "undefined" && /Mac/.test(navigator.platform));

  if (!isMac && !caps) {
    return null;
  }

  return (
    <SectionCard title={t("settings.appleMedia.title")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("settings.appleMedia.hint")}</p>
      {caps ? (
        <CapabilitiesBlock caps={caps} t={t} />
      ) : (
        <p className="text-sm text-secondary m-0">{t("settings.appleMedia.unavailable")}</p>
      )}
      {!isTauriDesktop() && caps?.stt && (
        <p className="text-xs text-secondary m-0 mt-3">{t("settings.appleMedia.cliHelperHint")}</p>
      )}
    </SectionCard>
  );
}
