import { useEffect, useState } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { api } from "../api";
import { AuthLayout } from "../components/AuthLayout";
import { useAuth } from "../hooks/useAuth";
import { useT } from "../i18n/context";
import {
  openAnycodeDeepLink,
  resolveDeviceRedirectUri,
} from "../lib/deviceLink";
import { SITE_PATHS } from "@anycode/site-urls";

export function RegisterPage() {
  const nav = useNavigate();
  const loc = useLocation();
  const { setToken } = useAuth();
  const t = useT();
  const search = new URLSearchParams(loc.search);
  const deviceCode = search.get("device_code");
  const redirectUri = search.get("redirect_uri");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [verificationCode, setVerificationCode] = useState("");
  const [privacyConsent, setPrivacyConsent] = useState(false);
  const [resendIn, setResendIn] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [handoffLink, setHandoffLink] = useState<string | null>(null);
  const [handoffNote, setHandoffNote] = useState<string | null>(null);

  useEffect(() => {
    if (resendIn <= 0) return;
    const timer = window.setTimeout(() => setResendIn((value) => value - 1), 1000);
    return () => window.clearTimeout(timer);
  }, [resendIn]);

  const sendCode = async () => {
    setError(null);
    try {
      await api.sendRegistrationCode(email);
      setResendIn(60);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setPending(true);
    try {
      const res = await api.register({
        email,
        password,
        display_name: displayName || email.split("@")[0] || "User",
        verification_code: verificationCode,
        privacy_consent: privacyConsent,
        consent_version: "2026-07-10",
      });
      setToken(res.token);
      if (deviceCode) {
        const deepLink = resolveDeviceRedirectUri(deviceCode, redirectUri);
        setHandoffLink(deepLink);
        try {
          await api.approveDeviceLink(deviceCode);
          setHandoffNote(t("devices.approved"));
        } catch (approveErr) {
          const message = approveErr instanceof Error ? approveErr.message : String(approveErr);
          if (/identity|实名|verify|already|approved|关联/i.test(message)) {
            nav(`/console/settings?code=${encodeURIComponent(deviceCode)}`, { replace: true });
            return;
          }
          setHandoffNote(message);
          return;
        }
        openAnycodeDeepLink(deepLink, { replacePage: false });
        window.setTimeout(() => nav("/console", { replace: true }), 600);
        return;
      }
      nav("/console/plans");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setPending(false);
    }
  };

  if (handoffLink) {
    return (
      <AuthLayout title={t("auth.registerTitle")} subtitle={t("devices.redirecting")}>
        <div className="device-link-panel">
          <p className="muted">{handoffNote ?? t("devices.openingDesktop")}</p>
          <p className="muted">{t("devices.clickDeepLink")}</p>
          <button
            type="button"
            className="btn btn-primary btn-block"
            onClick={() => nav("/console", { replace: true })}
          >
            {t("devices.enterConsole")}
          </button>
          <a
            className="btn btn-block"
            href={handoffLink}
            style={{ marginTop: "0.5rem" }}
            onClick={(e) => {
              e.preventDefault();
              openAnycodeDeepLink(handoffLink, { replacePage: true });
            }}
          >
            {t("devices.openDeepLink")}
          </a>
          <p className="muted auth-switch">{t("devices.redirectingConsole")}</p>
        </div>
      </AuthLayout>
    );
  }

  return (
    <AuthLayout title={t("auth.registerTitle")} subtitle={t("auth.registerSubtitle")}>
      <form className="auth-form auth-form--register" onSubmit={submit}>
        <div className="field-group">
          <label className="field-label" htmlFor="register-name">
            {t("auth.displayName")}
          </label>
          <input
            id="register-name"
            className="auth-input"
            placeholder={t("auth.displayNamePlaceholder")}
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
          />
        </div>

        <div className="field-group">
          <label className="field-label" htmlFor="register-email">
            {t("auth.email")}
          </label>
          <input
            id="register-email"
            className="auth-input"
            placeholder="you@example.com"
            type="email"
            autoComplete="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
          />
        </div>

        <div className="field-group">
          <label className="field-label" htmlFor="register-code">
            {t("auth.verificationCode")}
          </label>
          <div className="verification-row">
            <input
              id="register-code"
              className="auth-input"
              inputMode="numeric"
              autoComplete="one-time-code"
              pattern="[0-9]{6}"
              maxLength={6}
              value={verificationCode}
              onChange={(e) => setVerificationCode(e.target.value)}
              required
            />
            <button
              className="btn btn-secondary verification-send-btn"
              type="button"
              disabled={!email || resendIn > 0}
              onClick={() => void sendCode()}
            >
              {resendIn > 0 ? `${resendIn}s` : t("auth.sendCode")}
            </button>
          </div>
        </div>

        <div className="field-group">
          <label className="field-label" htmlFor="register-password">
            {t("auth.password")}
          </label>
          <input
            id="register-password"
            className="auth-input"
            placeholder={t("auth.passwordMinPlaceholder")}
            type="password"
            autoComplete="new-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            minLength={8}
            required
          />
        </div>

        <div className="auth-legal-panel">
          <label className="consent-row">
            <input
              type="checkbox"
              checked={privacyConsent}
              onChange={(e) => setPrivacyConsent(e.target.checked)}
              required
            />
            <span>
              {t("auth.consentPrefix")}
              <Link to={SITE_PATHS.legalUserAgreement} target="_blank" rel="noreferrer">
                {t("legal.userAgreementLink")}
              </Link>
              {t("auth.consentAnd")}
              <Link to={SITE_PATHS.legalPrivacy} target="_blank" rel="noreferrer">
                {t("legal.privacyLink")}
              </Link>
              {t("auth.consentSuffix")}
            </span>
          </label>
          <p className="auth-consent-note muted">{t("auth.algorithmNotice")}</p>
        </div>

        {error && (
          <p className="form-error" role="alert">
            {error}
          </p>
        )}
        <button className="btn btn-primary btn-block auth-submit" type="submit" disabled={pending}>
          {pending ? t("auth.signingUp") : t("auth.createAccount")}
        </button>
      </form>
      <p className="muted auth-switch">
        {t("auth.hasAccount")}{" "}
        <Link to="/login">{t("auth.loginLink")}</Link>
      </p>
    </AuthLayout>
  );
}
