import { Navigate, Route, Routes } from "react-router-dom";
import { getToken } from "./api";
import Layout from "./components/Layout";
import LoginPage from "./pages/LoginPage";
import PoolPage from "./pages/PoolPage";
import ModelsPage from "./pages/ModelsPage";
import UsagePage from "./pages/UsagePage";
import HealthPage from "./pages/HealthPage";
import IdentityReviewsPage from "./pages/IdentityReviewsPage";
import PlansPage from "./pages/PlansPage";

function RequireAuth({ children }: { children: React.ReactNode }) {
  if (!getToken()) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

export default function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route
        path="/"
        element={
          <RequireAuth>
            <Layout />
          </RequireAuth>
        }
      >
        <Route index element={<Navigate to="/pool" replace />} />
        <Route path="pool" element={<PoolPage />} />
        <Route path="models" element={<ModelsPage />} />
        <Route path="plans" element={<PlansPage />} />
        <Route path="usage" element={<UsagePage />} />
        <Route path="health" element={<HealthPage />} />
        <Route path="identity" element={<IdentityReviewsPage />} />
      </Route>
    </Routes>
  );
}
