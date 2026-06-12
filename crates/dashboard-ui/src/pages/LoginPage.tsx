import { useEffect, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Icon } from "@/components/Icon";
import { useAuth } from "@/auth/context";
import { useI18n } from "@/i18n/context";
import { LanguageSwitcher } from "@/components/UserMenu";
import { ThemeToggle } from "@/components/ThemeToggle";
import { BrandMark } from "@/components/BrandMark";

export function LoginPage() {
  const { t } = useI18n();
  const { login, authenticated } = useAuth();
  const navigate = useNavigate();
  const [email, setEmail] = useState("local@anycode");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  useEffect(() => {
    if (authenticated) {
      void navigate({ to: "/" });
    }
  }, [authenticated, navigate]);

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setPending(true);
    try {
      await login(email, password);
      void navigate({ to: "/" });
    } catch {
      setError(t("auth.loginFailed"));
    } finally {
      setPending(false);
    }
  };

  const onLocalTrusted = async () => {
    setError(null);
    setPending(true);
    try {
      await login("local@anycode", "");
      void navigate({ to: "/" });
    } catch {
      setError(t("auth.loginFailed"));
    } finally {
      setPending(false);
    }
  };

  return (
    <div className="h-full min-h-0 overflow-y-auto bg-background flex items-center justify-center p-6">
      <div className="w-full max-w-md">
        <div className="flex items-center justify-between mb-8">
          <BrandMark size="sm" showTitle variant="login" />
          <div className="flex items-center gap-2">
            <LanguageSwitcher />
            <ThemeToggle />
          </div>
        </div>

        <div className="bg-surface-container-lowest border border-outline-variant rounded-lg p-8 shadow-sm">
          <h2 className="text-lg font-semibold m-0 mb-1">{t("auth.loginTitle")}</h2>
          <p className="text-sm text-secondary m-0 mb-6">{t("auth.loginSubtitle")}</p>

          <form onSubmit={onSubmit} className="flex flex-col gap-4">
            <label className="flex flex-col gap-1 text-sm">
              <span className="text-secondary font-medium">{t("auth.email")}</span>
              <input
                className="dw-input"
                type="email"
                autoComplete="username"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                required
              />
            </label>
            <label className="flex flex-col gap-1 text-sm">
              <span className="text-secondary font-medium">{t("auth.password")}</span>
              <input
                className="dw-input"
                type="password"
                autoComplete="current-password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </label>
            {error && <div className="dw-alert-error">{error}</div>}
            <button type="submit" className="dw-btn-primary w-full justify-center py-2.5" disabled={pending}>
              {pending ? t("common.loading") : t("auth.signIn")}
            </button>
          </form>
          <p className="text-xs text-secondary mt-4 mb-3">{t("auth.loginHint")}</p>
          <button
            type="button"
            className="dw-btn-secondary w-full justify-center py-2"
            disabled={pending}
            onClick={() => void onLocalTrusted()}
          >
            <Icon name="verified_user" size={16} />
            {t("auth.continueLocal")}
          </button>
        </div>
      </div>
    </div>
  );
}
