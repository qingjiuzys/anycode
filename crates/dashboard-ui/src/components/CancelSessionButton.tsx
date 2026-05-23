import { useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { useT } from "@/i18n/context";

export function CancelSessionButton({
  sessionId,
  status,
  compact,
}: {
  sessionId: string;
  status: string;
  compact?: boolean;
}) {
  const t = useT();
  const queryClient = useQueryClient();
  const cancelRun = useMutation({
    mutationFn: () => api.cancelSession(sessionId),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
      queryClient.invalidateQueries({ queryKey: ["running-sessions"] });
      if (data.live_signal) {
        window.alert(t("session.cancelLiveSignal"));
      }
    },
  });

  if (status !== "running") return null;

  return (
    <button
      type="button"
      className={compact ? "dw-btn-secondary text-xs py-0.5 px-2" : "dw-btn-secondary text-sm"}
      disabled={cancelRun.isPending}
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        if (window.confirm(t("session.cancelConfirm"))) {
          cancelRun.mutate();
        }
      }}
    >
      {cancelRun.isPending ? t("session.cancelling") : t("session.cancelRun")}
    </button>
  );
}
