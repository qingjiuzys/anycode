import { useEffect, useState } from "react";
import { fetchUsageOverview, type UsageRow } from "../api";

export default function UsagePage() {
  const [rows, setRows] = useState<UsageRow[]>([]);

  useEffect(() => {
    fetchUsageOverview().then((r) => setRows(r.usage));
  }, []);

  return (
    <div>
      <h1>近 30 天用量（按组织）</h1>
      <div className="ops-table-wrap">
        <table>
          <thead>
            <tr>
              <th>组织 ID</th>
              <th>Tokens</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r) => (
              <tr key={r.organization_id}>
                <td>{r.organization_id}</td>
                <td>{r.total_tokens.toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
