import { useWorkbenchTerminal } from "../hooks/useWorkbenchTerminal";

type Props = {
  projectId: string;
  active: boolean;
};

export function TerminalPanel({ projectId, active }: Props) {
  const { containerRef } = useWorkbenchTerminal(projectId, active);

  return (
    <div
      ref={containerRef}
      className="h-full min-h-[200px] p-1 bg-[#0d1117] overflow-hidden"
    />
  );
}
