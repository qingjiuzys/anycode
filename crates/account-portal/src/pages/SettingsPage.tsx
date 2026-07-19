import { useCallback, useEffect, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { api } from "../api";
import { ConsolePage } from "../components/ConsolePage";
import { formatMessage, useT } from "../i18n/context";
import { anycodeDeepLinkForCode, openAnycodeDeepLink } from "../lib/deviceLink";

type Device = { id: string; device_name: string; last_seen_at: string; revoked: boolean };
type Identity = {
  status: string;
  legal_name_masked: string | null;
  id_number_masked: string | null;
  rejection_reason: string | null;
};

function deepLinkForCode(code: string): string {
  return anycodeDeepLinkForCode(code);
}

function identityStatusLabel(status: string | undefined, t: (key: string) => string): string {
  switch (status) {
    case "approved":
      return t("identity.statusApproved");
    case "rejected":
      return t("identity.statusRejected");
    case "pending":
      return t("identity.statusPending");
    default:
      return t("identity.statusUnverified");
  }
}

export function SettingsPage() {
  const [params] = useSearchParams();
  const t = useT();
  const [email, setEmail] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [profileDraft, setProfileDraft] = useState("");
  const [profileMessage, setProfileMessage] = useState<string | null>(null);
  const [devices, setDevices] = useState<Device[]>([]);
  const [identity, setIdentity] = useState<Identity | null>(null);
  const [identityApproved, setIdentityApproved] = useState(false);
  const [legalName, setLegalName] = useState("");
  const [idNumber, setIdNumber] = useState("");
  const [identityMessage, setIdentityMessage] = useState<string | null>(null);
  const [linkMsg, setLinkMsg] = useState<string | null>(null);
  const [manualCode, setManualCode] = useState("");
  const [approvedDeepLink, setApprovedDeepLink] = useState<string | null>(null);
  const [linkResult, setLinkResult] = useState<{
    device_code: string;
    deep_link: string;
    verification_uri: string;
  } | null>(null);
  const autoApproveAttempted = useRef(false);

  const pendingDeviceCode = params.get("code");

  const refreshDevices = () => {
    void api.devices().then((r) => setDevices(r.devices)).catch(() => setDevices([]));
  };

  const refreshIdentity = useCallback(() => {
    void api
      .identityStatus()
      .then((r) => {
        setIdentity(r.identity);
        setIdentityApproved(r.identity.status === "approved");
      })
      .catch(() => setIdentity(null));
  }, []);

  const approvePendingDevice = useCallback(
    async (code: string) => {
      try {
        await api.approveDeviceLink(code);
        const link = deepLinkForCode(code);
        setApprovedDeepLink(link);
        setLinkMsg(t("devices.approved"));
        refreshDevices();
        // Best-effort return to desktop (may need the visible button if blocked).
        openAnycodeDeepLink(link);
      } catch (error) {
        setLinkMsg(error instanceof Error ? error.message : String(error));
      }
    },
    [refreshDevices, t],
  );

  useEffect(() => {
    void api.me().then((r) => {
      setEmail(r.user.email);
      setDisplayName(r.user.display_name);
      setProfileDraft(r.user.display_name);
      setIdentityApproved(r.identity_status === "approved");
    });
    refreshDevices();
    refreshIdentity();
    if (pendingDeviceCode) {
      setLinkMsg(formatMessage(t("devices.linkCode"), { code: pendingDeviceCode }));
    }
  }, [pendingDeviceCode, refreshIdentity, t]);

  useEffect(() => {
    if (!pendingDeviceCode || !identityApproved || autoApproveAttempted.current) return;
    autoApproveAttempted.current = true;
    void approvePendingDevice(pendingDeviceCode);
  }, [pendingDeviceCode, identityApproved, approvePendingDevice]);

  const saveProfile = async (e: React.FormEvent) => {
    e.preventDefault();
    setProfileMessage(null);
    try {
      await api.updateProfile({ display_name: profileDraft });
      setDisplayName(profileDraft);
      setProfileMessage(t("settings.profileSaved"));
    } catch (error) {
      setProfileMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const submitIdentity = async (e: React.FormEvent) => {
    e.preventDefault();
    setIdentityMessage(null);
    try {
      await api.submitIdentity({ legal_name: legalName, id_number: idNumber });
      setLegalName("");
      setIdNumber("");
      setIdentityMessage(t("identity.verified"));
      refreshIdentity();
      const me = await api.me();
      setIdentityApproved(me.identity_status === "approved");
      if (pendingDeviceCode && me.identity_status === "approved") {
        await approvePendingDevice(pendingDeviceCode);
      }
    } catch (error) {
      setIdentityMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const openDesktop = async () => {
    setLinkMsg(null);
    setLinkResult(null);
    try {
      const res = await api.deviceLinkStart("anyCode Desktop");
      setLinkResult(res);
      setLinkMsg(t("devices.openingDesktop"));
      const popup = window.open(res.deep_link, "_blank", "noopener,noreferrer");
      if (!popup) {
        setLinkMsg(t("devices.clickDeepLink"));
      }
    } catch (e) {
      setLinkMsg(e instanceof Error ? e.message : String(e));
    }
  };

  const copyDeepLink = async (code: string) => {
    const link = deepLinkForCode(code);
    try {
      await navigator.clipboard.writeText(link);
      setLinkMsg(t("devices.deepLinkCopied"));
    } catch {
      setLinkMsg(link);
    }
  };

  const approveManualCode = async () => {
    const code = manualCode.trim();
    if (!code) return;
    await approvePendingDevice(code);
    setManualCode("");
  };

  const showIdentityForm = identity?.status !== "approved";

  return (
    <ConsolePage title={t("console.settings")} description={t("settings.description")}>
      {pendingDeviceCode && !identityApproved && (
        <div className="card nx-device-hint" role="status">
          <p>{t("devices.identityRequired")}</p>
        </div>
      )}

      <div className="nx-settings-grid">
        <div className="card nx-profile-panel">
          <div className="nx-profile-panel__mark">
            {(displayName || email || "A").slice(0, 1).toUpperCase()}
          </div>
          <div>
            <span className="nx-section-label">{t("settings.profileLabel")}</span>
            <h3>{t("settings.profileTitle")}</h3>
            <dl className="settings-dl">
              <div>
                <dt>{t("auth.email")}</dt>
                <dd>{email || "—"}</dd>
              </div>
            </dl>
            <form className="auth-form" onSubmit={saveProfile}>
              <label className="field-label" htmlFor="profile-name">
                {t("auth.displayName")}
              </label>
              <input
                id="profile-name"
                value={profileDraft}
                onChange={(e) => setProfileDraft(e.target.value)}
                required
              />
              <button className="btn btn-secondary" type="submit">
                {t("settings.saveProfile")}
              </button>
            </form>
            {profileMessage && (
              <p className="form-note" role="status">
                {profileMessage}
              </p>
            )}
          </div>
        </div>

        <div className="card nx-identity-panel">
          <div className="nx-section-heading">
            <div>
              <span className="nx-section-label">{t("identity.sectionLabel")}</span>
              <h3>{t("identity.title")}</h3>
            </div>
            <strong>{identityStatusLabel(identity?.status, t)}</strong>
          </div>
          <p className="muted">{t("identity.description")}</p>
          {identity?.legal_name_masked && (
            <p className="form-note">
              {identity.legal_name_masked} · {identity.id_number_masked}
            </p>
          )}
          {identity?.rejection_reason && (
            <p className="form-error" role="alert">
              {identity.rejection_reason}
            </p>
          )}
          {showIdentityForm && (
            <form className="auth-form" onSubmit={submitIdentity}>
              <label className="field-label" htmlFor="identity-name">
                {t("identity.legalName")}
              </label>
              <input
                id="identity-name"
                value={legalName}
                onChange={(e) => setLegalName(e.target.value)}
                required
              />
              <label className="field-label" htmlFor="identity-number">
                {t("identity.idNumber")}
              </label>
              <input
                id="identity-number"
                value={idNumber}
                onChange={(e) => setIdNumber(e.target.value)}
                minLength={18}
                maxLength={18}
                autoComplete="off"
                required
              />
              <button className="btn btn-primary" type="submit">
                {t("identity.submit")}
              </button>
            </form>
          )}
          {identityMessage && (
            <p className="form-note" role="status">
              {identityMessage}
            </p>
          )}
        </div>
      </div>

      <div className="card device-cta-card nx-device-command">
        <div>
          <span className="nx-section-label">{t("devices.sectionLabel")}</span>
          <h3>{t("settings.devicesTitle")}</h3>
          <p className="muted">{t("devices.ctaHint")}</p>
        </div>
        <div className="nx-device-command__actions">
          <button className="btn btn-primary" type="button" onClick={() => void openDesktop()}>
            {t("devices.openDesktop")}
          </button>
          {pendingDeviceCode && !approvedDeepLink && identityApproved && (
            <button
              className="btn btn-secondary"
              type="button"
              onClick={() => void approvePendingDevice(pendingDeviceCode)}
            >
              {t("devices.approve")}
            </button>
          )}
        </div>
        {!pendingDeviceCode && (
          <div className="device-link-panel">
            <p className="muted">{t("devices.pasteCodeHint")}</p>
            <div className="field-group" style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
              <input
                className="auth-input"
                placeholder="dev_..."
                value={manualCode}
                onChange={(e) => setManualCode(e.target.value)}
                aria-label={t("devices.manualCodeLabel")}
              />
              <button
                className="btn btn-secondary"
                type="button"
                disabled={!manualCode.trim() || !identityApproved}
                onClick={() => void approveManualCode()}
              >
                {t("devices.approve")}
              </button>
            </div>
          </div>
        )}
        {approvedDeepLink && (
          <div className="device-link-panel">
            <p className="muted">{t("devices.clickDeepLink")}</p>
            <a className="btn btn-primary" href={approvedDeepLink}>
              {t("devices.openDeepLink")}
            </a>
          </div>
        )}
        {linkResult && (
          <div className="device-link-panel">
            <p className="muted">{t("devices.clickDeepLink")}</p>
            <a className="btn btn-secondary" href={linkResult.deep_link}>
              {t("devices.openDeepLink")}
            </a>
            <p className="form-note">
              {formatMessage(t("devices.linkCode"), { code: linkResult.device_code })}
            </p>
            <button
              className="btn btn-ghost btn-sm"
              type="button"
              onClick={() => void copyDeepLink(linkResult.device_code)}
            >
              {t("devices.copyDeepLink")}
            </button>
          </div>
        )}
        {linkMsg && <p className="form-note">{linkMsg}</p>}
      </div>

      <div className="card table-wrap nx-data-panel">
        <table>
          <thead>
            <tr>
              <th>{t("devices.colName")}</th>
              <th>{t("devices.colLastSeen")}</th>
              <th>{t("devices.colStatus")}</th>
              <th>{t("devices.colActions")}</th>
            </tr>
          </thead>
          <tbody>
            {devices.length === 0 && (
              <tr>
                <td colSpan={4} className="nx-table-empty">
                  {t("common.empty")}
                </td>
              </tr>
            )}
            {devices.map((d) => (
              <tr key={d.id}>
                <td>{d.device_name}</td>
                <td>{d.last_seen_at}</td>
                <td>{d.revoked ? t("devices.statusRevoked") : t("devices.statusActive")}</td>
                <td>
                  {!d.revoked && (
                    <button
                      className="btn btn-ghost btn-sm"
                      type="button"
                      onClick={() => void api.revokeDevice(d.id).then(refreshDevices)}
                    >
                      {t("devices.revoke")}
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </ConsolePage>
  );
}
