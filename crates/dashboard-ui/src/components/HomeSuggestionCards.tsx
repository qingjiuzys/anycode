import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

export function HomeSuggestionCards({ onNewProject }: { onNewProject: () => void }) {
  const t = useT();
  const qc = useQueryClient();
  const scan = useMutation({
    mutationFn: api.scanProjects,
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["projects"] });
      void qc.invalidateQueries({ queryKey: ["overview"] });
      void qc.invalidateQueries({ queryKey: ["bootstrap"] });
    },
  });

  type CardItem =
    | {
        key: string;
        icon: string;
        title: string;
        subtitle: string;
        onClick: () => void;
        disabled?: boolean;
        loading?: boolean;
      }
    | {
        key: string;
        icon: string;
        title: string;
        subtitle: string;
        to: "/settings";
      };

  const cards: CardItem[] = [
    {
      key: "scan",
      icon: "radar",
      title: t("home.suggestions.scanTitle"),
      subtitle: t("home.suggestions.scanSubtitle"),
      onClick: () => scan.mutate(),
      disabled: scan.isPending,
      loading: scan.isPending,
    },
    {
      key: "new",
      icon: "add",
      title: t("home.suggestions.newProjectTitle"),
      subtitle: t("home.suggestions.newProjectSubtitle"),
      onClick: onNewProject,
    },
    {
      key: "settings",
      icon: "settings",
      title: t("home.suggestions.settingsTitle"),
      subtitle: t("home.suggestions.settingsSubtitle"),
      to: "/settings",
    },
  ];

  return (
    <div className="dw-suggestion-cards">
      {cards.map((card) => {
        const title = "loading" in card && card.loading ? t("common.loading") : card.title;
        const inner = (
          <>
            <Icon name={card.icon} size={20} className="text-secondary shrink-0" />
            <div className="min-w-0">
              <p className="dw-suggestion-card__title">{title}</p>
              <p className="dw-suggestion-card__subtitle">{card.subtitle}</p>
            </div>
          </>
        );

        if ("to" in card) {
          return (
            <Link key={card.key} to={card.to} className="dw-suggestion-card no-underline">
              {inner}
            </Link>
          );
        }

        return (
          <button
            key={card.key}
            type="button"
            className="dw-suggestion-card"
            disabled={card.disabled}
            onClick={card.onClick}
          >
            {inner}
          </button>
        );
      })}
    </div>
  );
}
