export interface GithubIssueSummary {
  number: number;
  title: string;
  state: string;
  html_url: string;
  updated_at: string;
  labels: string[];
}

export interface LinearIssueSummary {
  identifier: string;
  title: string;
  state: string;
  url: string;
  updated_at: string;
  labels: string[];
}
