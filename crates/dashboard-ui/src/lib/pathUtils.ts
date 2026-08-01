export function basename(path: string): string {
  return path.split(/[/\\]/).pop() ?? path;
}

export function extension(path: string): string {
  const base = basename(path);
  const dot = base.lastIndexOf(".");
  if (dot <= 0) return "";
  return base.slice(dot + 1).toLowerCase();
}

export {
  isAbsolutePath,
  resolveDeliverableAbsPath,
  isProcessArtifactPath,
  isPrimaryDeliverableKind,
} from "@/lib/deliverablePath";
