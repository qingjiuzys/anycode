import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";
import { Link } from "@tanstack/react-router";
import { useWorkbenchBrowser } from "../hooks/useWorkbenchBrowser";

type Props = {
  projectId: string;
  active: boolean;
};

export function BrowserPanel({ projectId, active }: Props) {
  const t = useT();
  const { urlInput, setUrlInput, navigate, screenshot, createSession } = useWorkbenchBrowser(
    projectId,
    active,
  );

  return (
    <div className="flex flex-col h-full min-h-0">
      <form
        className="flex items-center gap-1 px-2 py-2 border-b border-outline-variant/60 shrink-0"
        onSubmit={(e) => {
          e.preventDefault();
          navigate.mutate(urlInput);
        }}
      >
        <input
          type="url"
          className="flex-1 min-w-0 text-xs px-2 py-1 rounded border border-outline-variant bg-surface-container-low"
          value={urlInput}
          onChange={(e) => setUrlInput(e.target.value)}
          placeholder="https://"
        />
        <button type="submit" className="dw-btn-secondary p-1.5" disabled={navigate.isPending}>
          <Icon name="arrow_forward" size={16} />
        </button>
      </form>

      {createSession.isError && (
        <div className="px-3 py-3 text-xs text-secondary">
          <p className="m-0 mb-2">{(createSession.error as Error).message}</p>
          <Link to="/settings" search={{ section: "notify" }} className="text-primary">
            {t("workbench.browserSetup")}
          </Link>
        </div>
      )}

      <div className="conv-browser-viewport flex-1 min-h-0 overflow-auto bg-surface-container-low p-2">
        {screenshot.data?.screenshot.image_base64 ? (
          <img
            src={`data:image/png;base64,${screenshot.data.screenshot.image_base64}`}
            alt={urlInput}
            className="w-full h-auto rounded border border-outline-variant/40"
          />
        ) : (
          <p className="text-xs text-secondary text-center py-8 m-0">
            {createSession.isPending ? t("common.loading") : t("workbench.browserEmpty")}
          </p>
        )}
      </div>
    </div>
  );
}
