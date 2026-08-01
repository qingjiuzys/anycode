/** Map raw connector preview errors to i18n keys under `settings.*`. */
export function connectorPreviewErrorKey(
  source: "github" | "linear",
  message: string,
): string {
  const m = message.toLowerCase();
  if (source === "github") {
    if (m.includes("owner/repo") || m.includes("expected owner")) {
      return "settings.connectorErrGithubRepo";
    }
    if (m.includes("401") || m.includes("403") || m.includes("bad credentials")) {
      return "settings.connectorErrGithubAuth";
    }
    if (m.includes("404") || m.includes("not found")) {
      return "settings.connectorErrGithubNotFound";
    }
    return "settings.connectorErrGithubGeneric";
  }
  if (m.includes("api key") || m.includes("linear api key")) {
    return "settings.connectorErrLinearKey";
  }
  if (m.includes("team_key") || m.includes("team_id")) {
    return "settings.connectorErrLinearTeam";
  }
  if (m.includes("401") || m.includes("403") || m.includes("authentication")) {
    return "settings.connectorErrLinearAuth";
  }
  return "settings.connectorErrLinearGeneric";
}
