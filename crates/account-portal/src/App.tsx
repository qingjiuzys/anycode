import { Navigate, Route, Routes, useLocation, useSearchParams } from "react-router-dom";
import { ConsoleLayout } from "./components/ConsoleLayout";
import { SiteFooter } from "./components/SiteFooter";
import { TopNav } from "./components/TopNav";
import { AuthProvider, useAuth } from "./hooks/useAuth";
import { I18nProvider } from "./i18n/context";
import { ApiKeysPage } from "./pages/ApiKeysPage";
import { BillingPage } from "./pages/BillingPage";
import { CaseDetailPage } from "./pages/CaseDetailPage";
import { ChangelogPage } from "./pages/ChangelogPage";
import { DownloadsPage } from "./pages/DownloadsPage";
import { FeaturesPage } from "./pages/FeaturesPage";
import { HomePage } from "./pages/HomePage";
import { HomePageDemo } from "./pages/HomePageDemo";
import { LoginPage } from "./pages/LoginPage";
import { MarketingPlansPage } from "./pages/MarketingPlansPage";
import { OverviewPage } from "./pages/OverviewPage";
import { PlansPage } from "./pages/PlansPage";
import { ProductPage } from "./pages/ProductPage";
import { RegisterPage } from "./pages/RegisterPage";
import { SettingsPage } from "./pages/SettingsPage";
import { TeamPage } from "./pages/TeamPage";
import { UsagePage } from "./pages/UsagePage";
import { AlgorithmDisclosurePage } from "./pages/legal/AlgorithmDisclosurePage";
import { PrivacyPolicyPage } from "./pages/legal/PrivacyPolicyPage";
import { UserAgreementPage } from "./pages/legal/UserAgreementPage";
import { DocsRoutes } from "./pages/DocsRoutes";
import { SITE_PATHS } from "@anycode/site-urls";

function RequireAuth({ children }: { children: React.ReactNode }) {
  const { authenticated, validating } = useAuth();
  const loc = useLocation();
  if (validating) {
    return <p className="muted console-meta">Loading…</p>;
  }
  if (!authenticated) {
    return <Navigate to={SITE_PATHS.login} replace state={{ from: loc.pathname + loc.search }} />;
  }
  return <>{children}</>;
}

function LegacyDeviceLinkRedirect() {
  const [params] = useSearchParams();
  const code = params.get("code");
  const suffix = code ? `?code=${encodeURIComponent(code)}` : "";
  return <Navigate to={`/console/settings${suffix}`} replace />;
}

function AppRoutes() {
  const { pathname } = useLocation();
  const isHome = pathname === "/" || pathname === "/home-classic";
  const isConsole = pathname.startsWith("/console");
  const isDocs = pathname.startsWith("/docs");
  const isMarketing = !isConsole;

  return (
    <div className={`app app--nx${isHome ? " app--home" : ""}${isDocs ? " app--docs" : ""}`}>
      <TopNav />
      <main
        className={`main${isHome ? " main--landing" : ""}${isConsole ? " main--console" : ""}${isDocs ? " main--docs" : ""}${isMarketing && !isHome && !isDocs ? " main--site" : ""}`}
      >
        <Routes>
          <Route path="/" element={<HomePageDemo />} />
          <Route path={SITE_PATHS.features} element={<FeaturesPage />} />
          <Route path={SITE_PATHS.product} element={<ProductPage />} />
          <Route path={SITE_PATHS.plans} element={<MarketingPlansPage />} />
          <Route path={SITE_PATHS.downloads} element={<DownloadsPage />} />
          <Route path={SITE_PATHS.changelog} element={<ChangelogPage />} />
          <Route path="/cases/:caseId" element={<CaseDetailPage />} />
          <Route path="/home-classic" element={<HomePage />} />
          <Route path={SITE_PATHS.login} element={<LoginPage />} />
          <Route path={SITE_PATHS.register} element={<RegisterPage />} />
          <Route path={SITE_PATHS.legalAlgorithmDisclosure} element={<AlgorithmDisclosurePage />} />
          <Route path={SITE_PATHS.legalUserAgreement} element={<UserAgreementPage />} />
          <Route path={SITE_PATHS.legalPrivacy} element={<PrivacyPolicyPage />} />
          <Route path="/docs/*" element={<DocsRoutes />} />

          <Route
            path="/console"
            element={
              <RequireAuth>
                <ConsoleLayout />
              </RequireAuth>
            }
          >
            <Route index element={<OverviewPage />} />
            <Route path="plans" element={<PlansPage />} />
            <Route path="usage" element={<UsagePage />} />
            <Route path="billing" element={<BillingPage />} />
            <Route path="api" element={<ApiKeysPage />} />
            <Route path="team" element={<TeamPage />} />
            <Route path="settings" element={<SettingsPage />} />
          </Route>

          <Route path="/models" element={<Navigate to="/console/usage" replace />} />
          <Route path="/billing" element={<Navigate to="/console/billing" replace />} />
          <Route path="/devices" element={<Navigate to="/console/settings" replace />} />
          <Route path="/devices/link" element={<LegacyDeviceLinkRedirect />} />
        </Routes>
      </main>
      {isMarketing && !isHome ? <SiteFooter /> : null}
    </div>
  );
}

export function App() {
  return (
    <I18nProvider>
      <AuthProvider>
        <AppRoutes />
      </AuthProvider>
    </I18nProvider>
  );
}
