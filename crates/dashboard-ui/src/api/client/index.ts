import { agentsClient } from "./agents";
import { coreClient } from "./core";
import { governanceClient } from "./governance";
import { projectsClient } from "./projects";
import { sessionsClient } from "./sessions";
import { mediaClient } from "./media";
import { settingsClient } from "./settings";
import { setupClient } from "./setup";

export type {
  ArtifactListOpts,
  AuthUser,
  EventListOpts,
  ProjectsListOpts,
  SessionListOpts,
} from "./shared";

export const api = {
  ...coreClient,
  ...agentsClient,
  ...projectsClient,
  ...sessionsClient,
  ...settingsClient,
  ...mediaClient,
  ...governanceClient,
  ...setupClient,
};
