import { useEffect, useState } from "react";
import { api } from "../api";
import { ConsolePage } from "../components/ConsolePage";
import { useT } from "../i18n/context";

type Member = { id: string; email: string; display_name: string; role: string };
type Peer = {
  user_id: string;
  display_name: string;
  email: string;
  device_name: string;
  version: string;
  online: boolean;
  last_seen: string;
};

export function TeamPage() {
  const t = useT();
  const [members, setMembers] = useState<Member[]>([]);
  const [peers, setPeers] = useState<Peer[]>([]);
  const [error, setError] = useState<string | null>(null);

  const load = () => {
    setError(null);
    void Promise.all([api.orgMembers(), api.teamPeers()])
      .then(([m, p]) => {
        setMembers(m.members ?? []);
        setPeers(p.peers ?? []);
      })
      .catch((err) => {
        setMembers([]);
        setPeers([]);
        setError(err instanceof Error ? err.message : String(err));
      });
  };

  useEffect(() => {
    load();
    const timer = window.setInterval(load, 15_000);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <ConsolePage title={t("console.team")} description={t("console.teamSubtitle")}>
      {error ? <p className="console-error">{error}</p> : null}
      <section className="console-section">
        <h2 className="console-section__title">{t("console.teamOnline")}</h2>
        {peers.length === 0 ? (
          <p className="console-meta">{t("console.teamOnlineEmpty")}</p>
        ) : (
          <ul className="console-list">
            {peers.map((p) => (
              <li key={p.user_id + p.device_name} className="console-list__item">
                <strong>{p.display_name || p.device_name}</strong>
                <span className="console-meta">{p.email}</span>
                <span className="console-meta">
                  v{p.version} · {new Date(p.last_seen).toLocaleString()}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
      <section className="console-section">
        <h2 className="console-section__title">{t("console.teamMembers")}</h2>
        <ul className="console-list">
          {members.map((m) => (
            <li key={m.id} className="console-list__item">
              <strong>{m.display_name || m.email}</strong>
              <span className="console-meta">{m.email}</span>
              <span className="console-meta">{m.role}</span>
            </li>
          ))}
        </ul>
      </section>
      <p className="console-meta">{t("console.teamHandoffHint")}</p>
    </ConsolePage>
  );
}
