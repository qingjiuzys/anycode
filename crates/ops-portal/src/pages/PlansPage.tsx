import { FormEvent, useEffect, useState } from "react";
import { fetchPlans, patchPlan, type CloudPlan } from "../api";

function fenToYuan(fen: number): string {
  return (fen / 100).toFixed(2);
}

function yuanToFen(yuan: string): number {
  const n = Number.parseFloat(yuan);
  if (!Number.isFinite(n)) return 0;
  return Math.round(n * 100);
}

export default function PlansPage() {
  const [plans, setPlans] = useState<CloudPlan[]>([]);
  const [editing, setEditing] = useState<CloudPlan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  async function reload() {
    const res = await fetchPlans();
    setPlans(res.plans);
  }

  useEffect(() => {
    reload().catch((e) => setError(String(e)));
  }, []);

  async function onSave(e: FormEvent) {
    e.preventDefault();
    if (!editing) return;
    setSaving(true);
    setError(null);
    try {
      await patchPlan(editing.id, {
        display_name: editing.display_name,
        description: editing.description,
        monthly_price_fen: editing.monthly_price_fen,
        yearly_price_fen: editing.yearly_price_fen,
        token_limit: editing.token_limit,
        api_key_limit: editing.api_key_limit,
        seat_limit: editing.seat_limit,
        quota_window_secs: editing.quota_window_secs,
        calls_per_window: editing.calls_per_window,
        hosted_models_enabled: editing.hosted_models_enabled,
        promo_label: editing.promo_label,
        featured: editing.featured,
        enabled: editing.enabled,
        sort_order: editing.sort_order,
      });
      setEditing(null);
      await reload();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div>
      <h1>套餐配置</h1>
      <p className="ops-muted">价格、配额与优惠标签会同步到 Portal / Desktop 目录接口。</p>
      {error && <p className="ops-error">{error}</p>}
      <div className="ops-table-wrap">
        <table>
          <thead>
            <tr>
              <th>ID</th>
              <th>名称</th>
              <th>月价（元）</th>
              <th>年价（元）</th>
              <th>Token</th>
              <th>API Key</th>
              <th>席位</th>
              <th>优惠标签</th>
              <th>推荐</th>
              <th>启用</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {plans.map((p) => (
              <tr key={p.id}>
                <td>{p.id}</td>
                <td>{p.display_name}</td>
                <td>{fenToYuan(p.monthly_price_fen)}</td>
                <td>{fenToYuan(p.yearly_price_fen)}</td>
                <td>{p.token_limit.toLocaleString()}</td>
                <td>{p.api_key_limit}</td>
                <td>{p.seat_limit}</td>
                <td>{p.promo_label ?? "—"}</td>
                <td>{p.featured ? "是" : "否"}</td>
                <td>{p.enabled ? "是" : "否"}</td>
                <td>
                  <button type="button" onClick={() => setEditing({ ...p })}>
                    编辑
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {editing && (
        <form className="ops-form" onSubmit={(e) => void onSave(e)}>
          <h2>编辑 {editing.id}</h2>
          <label>
            显示名
            <input
              value={editing.display_name}
              onChange={(e) => setEditing({ ...editing, display_name: e.target.value })}
            />
          </label>
          <label>
            描述
            <input
              value={editing.description ?? ""}
              onChange={(e) =>
                setEditing({ ...editing, description: e.target.value || null })
              }
            />
          </label>
          <label>
            月价（元）
            <input
              type="number"
              step="0.01"
              value={fenToYuan(editing.monthly_price_fen)}
              onChange={(e) =>
                setEditing({
                  ...editing,
                  monthly_price_fen: yuanToFen(e.target.value),
                })
              }
            />
          </label>
          <label>
            年价（元）
            <input
              type="number"
              step="0.01"
              value={fenToYuan(editing.yearly_price_fen)}
              onChange={(e) =>
                setEditing({
                  ...editing,
                  yearly_price_fen: yuanToFen(e.target.value),
                })
              }
            />
          </label>
          <label>
            Token 限额
            <input
              type="number"
              value={editing.token_limit}
              onChange={(e) =>
                setEditing({
                  ...editing,
                  token_limit: Number.parseInt(e.target.value, 10) || 0,
                })
              }
            />
          </label>
          <label>
            API Key 限额
            <input
              type="number"
              value={editing.api_key_limit}
              onChange={(e) =>
                setEditing({
                  ...editing,
                  api_key_limit: Number.parseInt(e.target.value, 10) || 0,
                })
              }
            />
          </label>
          <label>
            席位
            <input
              type="number"
              value={editing.seat_limit}
              onChange={(e) =>
                setEditing({
                  ...editing,
                  seat_limit: Number.parseInt(e.target.value, 10) || 0,
                })
              }
            />
          </label>
          <label>
            5h 窗口（秒）
            <input
              type="number"
              value={editing.quota_window_secs}
              onChange={(e) =>
                setEditing({
                  ...editing,
                  quota_window_secs: Number.parseInt(e.target.value, 10) || 0,
                })
              }
            />
          </label>
          <label>
            窗口内调用次数
            <input
              type="number"
              value={editing.calls_per_window}
              onChange={(e) =>
                setEditing({
                  ...editing,
                  calls_per_window: Number.parseInt(e.target.value, 10) || 0,
                })
              }
            />
          </label>
          <label>
            优惠标签
            <input
              value={editing.promo_label ?? ""}
              onChange={(e) =>
                setEditing({
                  ...editing,
                  promo_label: e.target.value.trim() ? e.target.value.trim() : null,
                })
              }
              placeholder="如：推荐"
            />
          </label>
          <label>
            排序
            <input
              type="number"
              value={editing.sort_order}
              onChange={(e) =>
                setEditing({
                  ...editing,
                  sort_order: Number.parseInt(e.target.value, 10) || 0,
                })
              }
            />
          </label>
          <label className="ops-checkbox">
            <input
              type="checkbox"
              checked={editing.featured}
              onChange={(e) => setEditing({ ...editing, featured: e.target.checked })}
            />
            推荐（featured）
          </label>
          <label className="ops-checkbox">
            <input
              type="checkbox"
              checked={editing.enabled}
              onChange={(e) => setEditing({ ...editing, enabled: e.target.checked })}
            />
            启用
          </label>
          <label className="ops-checkbox">
            <input
              type="checkbox"
              checked={editing.hosted_models_enabled}
              onChange={(e) =>
                setEditing({ ...editing, hosted_models_enabled: e.target.checked })
              }
            />
            托管模型
          </label>
          <div className="ops-form-actions">
            <button type="submit" disabled={saving}>
              {saving ? "保存中…" : "保存"}
            </button>
            <button type="button" onClick={() => setEditing(null)}>
              取消
            </button>
          </div>
        </form>
      )}
    </div>
  );
}
