import { ExternalNavLink } from "@/components/ExternalNavLink";
import { useT } from "@/i18n/context";

export type GuideStep = {
  titleKey: string;
  bodyKey: string;
  linkHref?: string;
  linkLabelKey?: string;
};

export function ChannelGuideSteps({
  steps,
  prefix,
}: {
  steps: GuideStep[];
  prefix: "telegram" | "discord";
}) {
  const t = useT();
  return (
    <ol className="channel-guide-steps m-0 pl-5 mb-4 space-y-3">
      {steps.map((step, i) => (
        <li key={step.titleKey} className="text-sm">
          <strong className="block mb-0.5">
            {i + 1}. {t(`channels.guide.${prefix}.${step.titleKey}`)}
          </strong>
          <span className="text-secondary">{t(`channels.guide.${prefix}.${step.bodyKey}`)}</span>
          {step.linkHref && step.linkLabelKey && (
            <>
              {" "}
              <ExternalNavLink href={step.linkHref} className="dw-link text-sm">
                {t(`channels.guide.${prefix}.${step.linkLabelKey}`)}
              </ExternalNavLink>
            </>
          )}
        </li>
      ))}
    </ol>
  );
}

export const TELEGRAM_GUIDE_STEPS: GuideStep[] = [
  { titleKey: "step1Title", bodyKey: "step1Body", linkHref: "https://t.me/BotFather", linkLabelKey: "step1Link" },
  { titleKey: "step2Title", bodyKey: "step2Body" },
  { titleKey: "step3Title", bodyKey: "step3Body" },
  { titleKey: "step4Title", bodyKey: "step4Body" },
];

export const DISCORD_GUIDE_STEPS: GuideStep[] = [
  {
    titleKey: "step1Title",
    bodyKey: "step1Body",
    linkHref: "https://discord.com/developers/applications",
    linkLabelKey: "step1Link",
  },
  { titleKey: "step2Title", bodyKey: "step2Body" },
  { titleKey: "step3Title", bodyKey: "step3Body" },
  { titleKey: "step4Title", bodyKey: "step4Body" },
  { titleKey: "step5Title", bodyKey: "step5Body" },
];
