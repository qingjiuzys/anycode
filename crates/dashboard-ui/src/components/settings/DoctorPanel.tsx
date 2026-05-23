import type { DoctorReport } from "@/api/types";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";
import {
  translateDoctorCheckId,
  translateDoctorMessage,
  translateDoctorStep,
} from "@/i18n/doctorTranslate";

export function DoctorPanel({ doctor }: { doctor?: DoctorReport }) {
  const t = useT();
  if (!doctor) {
    return <p className="text-sm text-secondary">{t("settings.doctorLoading")}</p>;
  }

  return (
    <>
      <SectionCard title={t("settings.doctorPanel.status")}>
        <div className="flex items-center gap-2 mb-2">
          <StatusBadge status={doctor.status === "ok" ? "ok" : doctor.status} />
          <span className="text-sm text-secondary">{doctor.generated_at}</span>
        </div>
        {doctor.status === "ok" && (doctor.checks ?? []).every((c) => c.status === "ok") && (
          <p className="text-sm text-secondary m-0">{t("settings.doctorPanel.allOk")}</p>
        )}
      </SectionCard>

      {(doctor.checks ?? []).length > 0 && (
        <SectionCard title={t("settings.doctorPanel.checks")} noPadding>
          <div className="overflow-x-auto">
            <table className="dw-table">
              <thead>
                <tr>
                  <th>{t("common.id")}</th>
                  <th>{t("common.status")}</th>
                  <th>{t("common.details")}</th>
                </tr>
              </thead>
              <tbody>
                {doctor.checks.map((c) => (
                  <tr key={c.id}>
                    <td className="font-code text-xs">{translateDoctorCheckId(t, c.id)}</td>
                    <td>
                      <StatusBadge status={c.status} />
                    </td>
                    <td className="text-secondary text-sm">
                      {translateDoctorMessage(t, c.message)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </SectionCard>
      )}

      {(doctor.next_steps ?? []).length > 0 && (
        <SectionCard title={t("settings.doctorPanel.nextSteps")}>
          <ul className="m-0 pl-5 text-sm space-y-1">
            {doctor.next_steps!.map((s) => (
              <li key={s}>{translateDoctorStep(t, s)}</li>
            ))}
          </ul>
        </SectionCard>
      )}
    </>
  );
}
