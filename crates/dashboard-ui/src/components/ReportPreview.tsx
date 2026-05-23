import type { ReportDocument } from "@/api/types";
import { useClipboard } from "@/hooks/useClipboard";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

interface Props {
  report: ReportDocument | null;
  loading?: boolean;
}

export function ReportPreview({ report, loading }: Props) {
  const t = useT();
  const { copy, copied } = useClipboard();

  if (loading) {
    return <p className="text-sm text-secondary">{t("reports.generating")}</p>;
  }
  if (!report) {
    return null;
  }

  const download = () => {
    const blob = new Blob([report.markdown], { type: "text/markdown" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `anycode-report-${report.scope}-${report.id.slice(0, 8)}.md`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <SectionCard
      title={report.title}
      action={
        <div className="flex gap-2">
          <button type="button" className="dw-btn-secondary" onClick={() => copy(report.markdown)}>
            {copied ? t("common.copied") : t("reports.copyMarkdown")}
          </button>
          <button type="button" className="dw-btn-secondary" onClick={download}>
            {t("reports.downloadMd")}
          </button>
        </div>
      }
    >
      <p className="text-sm text-secondary m-0 mb-4">
        {report.scope} · {t("conversations.trust")}: {report.trusted_status} · {report.generated_at}
      </p>
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 mb-4">
        <MiniStat label={t("reports.sessions")} value={report.summary.sessions} />
        <MiniStat label={t("reports.events")} value={report.summary.events} />
        <MiniStat label={t("reports.failedGates")} value={report.summary.failed_gates} />
        <MiniStat label={t("reports.artifacts")} value={report.summary.artifacts} />
      </div>
      <pre className="bg-surface-container-low border border-outline-variant rounded p-4 font-code text-xs overflow-auto max-h-[480px] whitespace-pre-wrap m-0">
        {report.markdown}
      </pre>
    </SectionCard>
  );
}

function MiniStat({ label, value }: { label: string; value: number }) {
  return (
    <div className="dw-stat-card">
      <div className="dw-stat-value">{value}</div>
      <div className="dw-stat-label">{label}</div>
    </div>
  );
}
