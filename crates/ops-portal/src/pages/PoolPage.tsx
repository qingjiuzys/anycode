import { FormEvent, useEffect, useState } from "react";
import {
  createAccount,
  fetchAccounts,
  patchAccount,
  type UpstreamAccount,
} from "../api";

export default function PoolPage() {
  const [accounts, setAccounts] = useState<UpstreamAccount[]>([]);
  const [name, setName] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function reload() {
    const res = await fetchAccounts();
    setAccounts(res.accounts);
  }

  useEffect(() => {
    reload().catch((e) => setError(String(e)));
  }, []);

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      await createAccount({
        name,
        api_key: apiKey,
        base_url: baseUrl || undefined,
      });
      setName("");
      setApiKey("");
      setBaseUrl("");
      await reload();
    } catch (err) {
      setError(err instanceof Error ? err.message : "创建失败");
    }
  }

  return (
    <div>
      <h1>Agnes 账号池</h1>
      <form className="ops-card ops-form" onSubmit={onCreate}>
        <h2>新增账号</h2>
        <label>
          名称
          <input value={name} onChange={(e) => setName(e.target.value)} required />
        </label>
        <label>
          API Key（仅写入，不可回读）
          <input value={apiKey} onChange={(e) => setApiKey(e.target.value)} required />
        </label>
        <label>
          Base URL（可选）
          <input
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
            placeholder="https://apihub.agnes-ai.com/v1/chat/completions"
          />
        </label>
        <button type="submit">添加</button>
      </form>
      {error && <p className="ops-error">{error}</p>}
      <div className="ops-table-wrap">
        <table>
          <thead>
            <tr>
              <th>名称</th>
              <th>状态</th>
              <th>权重</th>
              <th>失败</th>
              <th>冷却至</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            {accounts.map((a) => (
              <tr key={a.id}>
                <td>{a.name}</td>
                <td>{a.status}</td>
                <td>{a.weight}</td>
                <td>{a.failure_count}</td>
                <td>{a.cooldown_until ?? "—"}</td>
                <td>
                  <button
                    type="button"
                    onClick={() =>
                      patchAccount(a.id, {
                        status: a.status === "active" ? "disabled" : "active",
                      }).then(reload)
                    }
                  >
                    {a.status === "active" ? "禁用" : "启用"}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
