/** Translate backend doctor check messages and next steps */

export function translateDoctorMessage(t: (key: string) => string, message: string): string {
  const exact = t(`doctorChecks.${message}`);
  if (exact !== `doctorChecks.${message}`) return exact;

  const dbFound = message.match(/^Database found at (.+)$/);
  if (dbFound) {
    return t("doctorChecks.dbFound").replace("{path}", dbFound[1]!);
  }
  const dbMissing = message.match(/^Database not found at (.+) \(will be created on first run\)$/);
  if (dbMissing) {
    return t("doctorChecks.dbMissing").replace("{path}", dbMissing[1]!);
  }
  const loopback = message.match(/^Bound to loopback (.+)$/);
  if (loopback) {
    return t("doctorChecks.loopbackBound").replace("{addr}", loopback[1]!);
  }
  const nonLoopback = message.match(/^Non-loopback binding (.+) — API token required$/);
  if (nonLoopback) {
    return t("doctorChecks.nonLoopbackBound").replace("{addr}", nonLoopback[1]!);
  }
  const portAvail = message.match(/^Port (\d+) is available$/);
  if (portAvail) {
    return t("doctorChecks.portAvailable").replace("{port}", portAvail[1]!);
  }
  const portUsed = message.match(/^Port (\d+) is already in use$/);
  if (portUsed) {
    return t("doctorChecks.portInUse").replace("{port}", portUsed[1]!);
  }
  const uiFound = message.match(/^UI dist found: (.+)$/);
  if (uiFound) {
    return t("doctorChecks.uiFound").replace("{path}", uiFound[1]!);
  }

  return message;
}

export function translateDoctorStep(t: (key: string) => string, step: string): string {
  const exact = t(`doctorSteps.${step}`);
  if (exact !== `doctorSteps.${step}`) return exact;
  return step;
}

export function translateDoctorCheckId(t: (key: string) => string, id: string): string {
  const key = t(`doctorCheckIds.${id}`);
  return key !== `doctorCheckIds.${id}` ? key : id;
}
