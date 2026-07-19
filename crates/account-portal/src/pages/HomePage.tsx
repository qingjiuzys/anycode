import { HeroShowcase } from "../components/HeroShowcase";
import { FeatureSection } from "../components/FeatureSection";
import { PlanPreview } from "../components/PlanPreview";
import { FinalCta } from "../components/FinalCta";
import { LandingFooter } from "../components/LandingFooter";

export function HomePage() {
  return (
    <div className="landing lx-home">
      <HeroShowcase />
      <FeatureSection />
      <PlanPreview />
      <FinalCta />
      <LandingFooter />
    </div>
  );
}
