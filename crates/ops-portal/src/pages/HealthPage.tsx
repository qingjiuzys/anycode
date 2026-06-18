import { useEffect, useState } from "react";
import { fetchHealthEvents, type HealthEvent } from "../api";

export default function HealthPage() {
  const [events, setEvents] = useState<HealthEvent[]>([]);

  useEffect(() => {
    fetchHealthEvents().then((r) => setEvents(r.events));
  }, []);

  return (
    <div>
      <h1>上游健康事件</h1>
      <div className="ops-table-wrap">
        <table>
          <thead>
            <tr>
              <th>时间</th>
              <th>账号</th>
              <th>类型</th>
              <th>状态码</th>
              <th>消息</th>
            </tr>
          </thead>
          <tbody>
            {events.map((e) => (
              <tr key={e.id}>
                <td>{e.created_at}</td>
                <td>{e.account_id}</td>
                <td>{e.event_type}</td>
                <td>{e.status_code ?? "—"}</td>
                <td className="ops-truncate">{e.message ?? "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
