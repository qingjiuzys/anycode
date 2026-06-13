import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { HomeHeroComposer } from "@/components/HomeHeroComposer";
import { HomeSuggestionCards } from "@/components/HomeSuggestionCards";
import { NewProjectDialog } from "@/components/NewProjectDialog";
import { usePendingApprovalCounts } from "@/components/SecurityApprovalInbox";
import { useSseStatus } from "@/context/SseContext";
import { useT } from "@/i18n/context";

export function HomePage() {
  const t = useT();
  const sseStatus = useSseStatus();
  const [newProjectOpen, setNewProjectOpen] = useState(false);
  const [prompt, setPrompt] = useState("");
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const overview = useQuery({ queryKey: ["overview"], queryFn: api.overview });
  const projects = useQuery({
    queryKey: ["projects", "home-top"],
    queryFn: () => api.projects({ limit: 8, sort: "updated_at_desc" }),
  });
  const { pendingTotal } = usePendingApprovalCounts();

  if (health.isError) {
    return (
      <div className="dw-alert-error">
        {t("home.apiError")} <code className="font-code">anycode dashboard</code>
      </div>
    );
  }

  const list = projects.data?.projects ?? [];
  const ov = overview.data?.overview;

  return (
    <>
      <NewProjectDialog open={newProjectOpen} onClose={() => setNewProjectOpen(false)} />

      <section className="dw-hero dw-home-hero">
        <div className="hero-glow" aria-hidden />
        <div className="dw-home-hero__intro">
          <h1 className="dw-hero__title m-0">
            {t("home.hero.titleLead")}{" "}
            <span className="accent-text">{t("home.hero.titleAccent")}</span>
            {t("home.hero.titleRest") ? ` ${t("home.hero.titleRest")}` : ""}
          </h1>
          <p className="dw-home-hero__subtitle">{t("home.hero.subtitle")}</p>
        </div>
        <HomeHeroComposer
          sseStatus={sseStatus}
          projectOptions={list.map((p) => ({ id: p.id, name: p.name }))}
          blockedCount={ov?.sessions_blocked ?? 0}
          pendingCount={pendingTotal}
          budgetExceededCount={ov?.sessions_budget_exceeded ?? 0}
          prompt={prompt}
          onPromptChange={setPrompt}
        />
        <HomeSuggestionCards onSelectPrompt={setPrompt} />
      </section>
    </>
  );
}
