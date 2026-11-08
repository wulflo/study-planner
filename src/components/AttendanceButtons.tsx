import type { AttendanceStatus, ScheduleOccurrence } from "../lib/ipc";

const attendanceLabels: Record<AttendanceStatus, string> = {
  present: "Presente",
  absent: "Ausente",
  late: "Tarde",
  justified: "Justificado",
};

const attendanceShort: Record<AttendanceStatus, string> = {
  present: "P",
  absent: "A",
  late: "T",
  justified: "J",
};

const allAttendance: AttendanceStatus[] = [
  "present",
  "absent",
  "late",
  "justified",
];

export function AttendanceButtons({
  occurrence,
  busy,
  onChange,
}: {
  occurrence: ScheduleOccurrence;
  busy: boolean;
  onChange: (status: AttendanceStatus | null) => void;
}) {
  return (
    <fieldset
      className="attendance-row"
      aria-label={`Asistencia de «${occurrence.subjectName}» el ${occurrence.date}`}
    >
      {allAttendance.map((status) => {
        const active = occurrence.attendanceStatus === status;
        return (
          <button
            type="button"
            key={status}
            className={`attendance-button${active ? " is-active" : ""}`}
            title={attendanceLabels[status]}
            aria-label={`Marcar ${attendanceLabels[status].toLowerCase()} «${occurrence.subjectName}» ${occurrence.date}`}
            aria-pressed={active}
            onClick={() => onChange(active ? null : status)}
            disabled={busy}
          >
            {attendanceShort[status]}
          </button>
        );
      })}
    </fieldset>
  );
}
