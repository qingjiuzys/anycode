import { useT } from "@/i18n/context";

const SUGGESTIONS = [
  { key: "scaffold" as const, promptKey: "scaffoldPrompt" as const },
  { key: "fix" as const, promptKey: "fixPrompt" as const },
  { key: "automate" as const, promptKey: "automatePrompt" as const },
];

export function HomeSuggestionCards({ onSelectPrompt }: { onSelectPrompt: (prompt: string) => void }) {
  const t = useT();

  return (
    <div className="dw-suggestion-cards">
      {SUGGESTIONS.map(({ key, promptKey }) => (
        <button
          key={key}
          type="button"
          className="glass-card dw-suggestion-card"
          onClick={() => onSelectPrompt(t(`home.suggestions.${promptKey}`))}
        >
          <h3 className="dw-suggestion-card__title">{t(`home.suggestions.${key}Title`)}</h3>
          <p className="dw-suggestion-card__subtitle">{t(`home.suggestions.${key}Subtitle`)}</p>
        </button>
      ))}
    </div>
  );
}
