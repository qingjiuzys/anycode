import { agentsClient } from "./agents";
import { assetsClient } from "./assets";
import { coreClient } from "./core";
import { governanceClient } from "./governance";
import { projectsClient } from "./projects";
import { sessionsClient } from "./sessions";
import { mediaClient } from "./media";
import { settingsClient } from "./settings";
import { setupClient } from "./setup";
import { workbenchClient } from "./workbench";

export type {
  ArtifactListOpts,
  AuthUser,
  EventListOpts,
  ProjectsListOpts,
  SessionListOpts,
} from "./shared";
export type { AssetListOpts } from "./assets";

export const api = {
  ...coreClient,
  ...agentsClient,
  ...projectsClient,
  ...sessionsClient,
  ...assetsClient,
  ...settingsClient,
  ...mediaClient,
  ...governanceClient,
  ...setupClient,
  ...workbenchClient,
};
