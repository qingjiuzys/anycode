import { useEffect, useState } from "react";
import { fetchModels, type CloudModel } from "../api";

export default function ModelsPage() {
  const [models, setModels] = useState<CloudModel[]>([]);

  useEffect(() => {
    fetchModels().then((r) => setModels(r.models));
  }, []);

  return (
    <div>
      <h1>云端模型目录</h1>
      <div className="ops-table-wrap">
        <table>
          <thead>
            <tr>
              <th>ID</th>
              <th>名称</th>
              <th>Provider</th>
              <th>最低套餐</th>
            </tr>
          </thead>
          <tbody>
            {models.map((m) => (
              <tr key={m.id}>
                <td>{m.id}</td>
                <td>{m.display_name}</td>
                <td>{m.provider_id}</td>
                <td>{m.min_plan}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
