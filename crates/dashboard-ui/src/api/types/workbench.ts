export type FsEntryKind = "file" | "dir" | "symlink";

export interface FsEntry {
  name: string;
  path: string;
  kind: FsEntryKind;
  size?: number;
}

export interface FsReadResult {
  path: string;
  content: string;
  truncated: boolean;
  size: number;
  mime_hint: string;
}

export interface BrowserState {
  url: string;
  title: string;
}

export interface BrowserScreenshot {
  image_base64: string;
  viewport: { width: number; height: number };
}

export interface BrowserSessionInfo {
  session_id: string;
  project_id: string;
}

export type WorkbenchTab = "files" | "browser" | "terminal" | "artifacts" | "trace";
