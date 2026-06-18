import { NavLink, Outlet } from "react-router-dom";
import { clearToken } from "../api";
import { useNavigate } from "react-router-dom";

export default function Layout() {
  const navigate = useNavigate();
  return (
    <div className="ops-shell">
      <aside className="ops-sidebar">
        <div className="ops-brand">anyCode Ops</div>
        <nav>
          <NavLink to="/pool">账号池</NavLink>
          <NavLink to="/models">模型目录</NavLink>
          <NavLink to="/usage">用量概览</NavLink>
          <NavLink to="/health">健康事件</NavLink>
        </nav>
        <button
          type="button"
          className="ops-logout"
          onClick={() => {
            clearToken();
            navigate("/login");
          }}
        >
          退出
        </button>
      </aside>
      <main className="ops-main">
        <Outlet />
      </main>
    </div>
  );
}
