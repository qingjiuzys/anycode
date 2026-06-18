/** Optional props for pages rendered inside the control center overlay. */
export type EmbeddedPageProps = {
  embedded?: boolean;
  projectId?: string;
  artifactId?: string;
  skillId?: string;
  initialSearch?: Record<string, string>;
};
