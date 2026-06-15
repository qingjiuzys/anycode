import { useState } from "react";
import { SectionCard } from "@/components/ui/SectionCard";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

export function ServiceCloudLogin() {
  const t = useT();
  const { login, register, baseUrl } = useAccountCloud();
  const [mode, setMode] = useState<"login" | "register">("login");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const submit = async () => {
    setError(null);
    setPending(true);
    try {
      if (mode === "login") {
        await login(email, password);
      } else {
        await register(email, password, displayName || email.split("@")[0] || "User");
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setPending(false);
    }
  };

  return (
    <SectionCard title={t("service.cloud.signInTitle")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("service.cloud.signInBody")}</p>
      {baseUrl && (
        <p className="text-xs font-code text-secondary m-0 mb-4 break-all">{baseUrl}</p>
      )}
      <div className="flex gap-2 mb-4">
        <button
          type="button"
          className={`dw-btn-secondary text-sm ${mode === "login" ? "ring-1 ring-primary" : ""}`}
          onClick={() => setMode("login")}
        >
          {t("service.cloud.loginTab")}
        </button>
        <button
          type="button"
          className={`dw-btn-secondary text-sm ${mode === "register" ? "ring-1 ring-primary" : ""}`}
          onClick={() => setMode("register")}
        >
          {t("service.cloud.registerTab")}
        </button>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 max-w-xl">
        {mode === "register" && (
          <label className="flex flex-col gap-1 text-sm md:col-span-2">
            <span className="text-secondary">{t("common.name")}</span>
            <input className="dw-input" value={displayName} onChange={(e) => setDisplayName(e.target.value)} />
          </label>
        )}
        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary">{t("auth.email")}</span>
          <input
            className="dw-input"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary">{t("service.cloud.password")}</span>
          <input
            className="dw-input"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
      </div>
      {error && <p className="text-sm text-error mt-3 m-0">{error}</p>}
      <button type="button" className="dw-btn-primary mt-4" disabled={pending} onClick={() => void submit()}>
        {mode === "login" ? t("auth.signIn") : t("service.cloud.registerCta")}
      </button>
    </SectionCard>
  );
}
