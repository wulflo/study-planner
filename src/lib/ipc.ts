import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  name: string;
  version: string;
  schemaVersion: number;
  exportFormatVersion: number;
}

export interface SchemaCheck {
  schemaVersion: number;
  integrity: string;
  foreignKeyViolations: number;
  vaultId: string;
}

export interface BackupInfo {
  fileName: string;
  sha256: string;
  sizeBytes: number;
  schemaVersion: number;
  integrity: string;
  foreignKeyViolations: number;
  modifiedAtMillis: number;
}

export interface SnapshotInfo {
  date: string;
  fileName: string;
  sha256: string;
  sizeBytes: number;
  schemaVersion: number;
  integrity: string;
  foreignKeyViolations: number;
  createdAt: string;
}

export interface ExportInfo {
  fileName: string;
  sha256: string;
  sizeBytes: number;
  rows: number;
  snapshotAt: string;
}

export interface ImportOutcome {
  status: VaultStatus;
  fileName: string;
  rows: number;
  attachments: number;
  vaultId: string;
  snapshotAt: string;
}

export interface VaultStatus {
  configured: boolean;
  ready: boolean;
  name: string | null;
  device: string | null;
  schemaVersion: number | null;
  integrity: string | null;
  foreignKeyViolations: number | null;
  vaultId: string | null;
  hasData: boolean;
  backup: BackupInfo | null;
  snapshots: SnapshotInfo[];
  message: string | null;
}

export type TaskStatus = "pending" | "in_progress" | "done";

export interface Task {
  id: string;
  title: string;
  status: TaskStatus;
  createdAt: string;
  completedAt: string | null;
}

export const runningInTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}

export async function runSchemaSelfCheck(): Promise<SchemaCheck> {
  return invoke<SchemaCheck>("schema_self_check");
}

export async function getVaultStatus(): Promise<VaultStatus> {
  return invoke<VaultStatus>("get_vault_status");
}

/** Abre el selector nativo. Devuelve `null` si el usuario cancela. */
export async function chooseVault(): Promise<VaultStatus | null> {
  return invoke<VaultStatus | null>("choose_vault");
}

export async function createBackup(): Promise<VaultStatus> {
  return invoke<VaultStatus>("create_backup");
}

/** Abre el selector nativo. Devuelve `null` si el usuario cancela. */
export async function exportVault(): Promise<ExportInfo | null> {
  return invoke<ExportInfo | null>("export_vault");
}

/** Importa solo en una bóveda vacía. Devuelve `null` si se cancela. */
export async function importVault(): Promise<ImportOutcome | null> {
  return invoke<ImportOutcome | null>("import_vault");
}

export async function restoreMainBackup(): Promise<VaultStatus> {
  return invoke<VaultStatus>("restore_main_backup");
}

export async function listTasks(): Promise<Task[]> {
  return invoke<Task[]>("list_tasks");
}

export async function createTask(title: string): Promise<Task> {
  return invoke<Task>("create_task", { title });
}

export async function setTaskDone(id: string, done: boolean): Promise<Task> {
  return invoke<Task>("set_task_done", { id, done });
}

export async function deleteTask(id: string): Promise<void> {
  return invoke<void>("delete_task", { id });
}

// ---------------------------------------------------------------------------
// Estructura académica y horario
// ---------------------------------------------------------------------------

export type TermKind =
  | "semester"
  | "trimester"
  | "quarter"
  | "bimonth"
  | "annual"
  | "other";

export type Recurrence = "weekly" | "biweekly";
export type ExceptionKind = "cancelled" | "moved" | "extra";
export type RotationMode = "weekly_ab" | "parity" | "day_cycle";
export type OccurrenceKind = "scheduled" | "cancelled" | "moved" | "extra";
export type AttendanceStatus = "present" | "absent" | "late" | "justified";

export interface AcademicYear {
  id: string;
  name: string;
  startDate: string;
  endDate: string;
  createdAt: string;
  updatedAt: string;
}

export interface Term {
  id: string;
  yearId: string;
  name: string;
  kind: TermKind;
  startDate: string;
  endDate: string;
  createdAt: string;
  updatedAt: string;
}

export interface Subject {
  id: string;
  termId: string;
  name: string;
  code: string | null;
  color: string;
  teacher: string | null;
  room: string | null;
  credits: number | null;
  goalScoreCp: number | null;
  attendanceMinBp: number | null;
  createdAt: string;
  updatedAt: string;
}

export interface ClassSlot {
  id: string;
  subjectId: string;
  weekday: number;
  startTime: string;
  endTime: string;
  recurrence: Recurrence;
  rotationKey: string | null;
  validFrom: string | null;
  validTo: string | null;
  roomOverride: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface Holiday {
  id: string;
  termId: string | null;
  date: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export interface ClassException {
  id: string;
  classSlotId: string;
  date: string;
  kind: ExceptionKind;
  newStartTime: string | null;
  newEndTime: string | null;
  newRoom: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface AttendanceRecord {
  id: string;
  classSlotId: string;
  date: string;
  status: AttendanceStatus;
  note: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface RotationConfig {
  id: string;
  termId: string;
  mode: RotationMode;
  anchorDate: string;
  cycleLength: number;
  labels: string[];
  dayCycle: string[];
  createdAt: string;
  updatedAt: string;
}

export interface ScheduleOccurrence {
  date: string;
  weekday: number;
  slotId: string;
  subjectId: string;
  subjectName: string;
  subjectColor: string;
  startTime: string;
  endTime: string;
  room: string | null;
  kind: OccurrenceKind;
  attendanceStatus: AttendanceStatus | null;
}

export interface SlotOverlap {
  weekday: number;
  firstSlotId: string;
  secondSlotId: string;
  firstSubjectName: string;
  secondSubjectName: string;
  startTime: string;
  endTime: string;
}

export interface AttendanceSummary {
  subjectId: string;
  subjectName: string;
  subjectColor: string;
  present: number;
  absent: number;
  late: number;
  justified: number;
  counted: number;
  attended: number;
  percentBp: number | null;
  minBp: number | null;
  belowMin: boolean;
}

export interface WeekHoliday {
  date: string;
  name: string;
}

export interface WeekSchedule {
  termId: string;
  weekStart: string;
  weekEnd: string;
  rotationLabel: string | null;
  holidays: WeekHoliday[];
  occurrences: ScheduleOccurrence[];
  overlaps: SlotOverlap[];
  attendance: AttendanceSummary[];
}

export interface TermInput {
  name: string;
  kind: TermKind;
  startDate: string;
  endDate: string;
}

export interface SubjectInput {
  name: string;
  code: string | null;
  color: string;
  teacher: string | null;
  room: string | null;
  credits: number | null;
  attendanceMinBp: number | null;
}

export interface ClassSlotInput {
  weekday: number;
  startTime: string;
  endTime: string;
  recurrence: Recurrence;
  rotationKey: string | null;
  validFrom: string | null;
  validTo: string | null;
  roomOverride: string | null;
}

export interface RotationInput {
  mode: RotationMode;
  anchorDate: string;
  cycleLength: number;
  labels: string[];
  dayCycle: string[];
}

export async function listAcademicYears(): Promise<AcademicYear[]> {
  return invoke<AcademicYear[]>("list_academic_years");
}

export async function createAcademicYear(
  name: string,
  startDate: string,
  endDate: string,
): Promise<AcademicYear> {
  return invoke<AcademicYear>("create_academic_year", {
    name,
    startDate,
    endDate,
  });
}

export async function updateAcademicYear(
  id: string,
  name: string,
  startDate: string,
  endDate: string,
): Promise<AcademicYear> {
  return invoke<AcademicYear>("update_academic_year", {
    id,
    name,
    startDate,
    endDate,
  });
}

export async function deleteAcademicYear(id: string): Promise<void> {
  return invoke<void>("delete_academic_year", { id });
}

export async function listTerms(yearId: string | null): Promise<Term[]> {
  return invoke<Term[]>("list_terms", { yearId });
}

export async function createTerm(
  yearId: string,
  input: TermInput,
): Promise<Term> {
  return invoke<Term>("create_term", { yearId, input });
}

export async function updateTerm(id: string, input: TermInput): Promise<Term> {
  return invoke<Term>("update_term", { id, input });
}

export async function deleteTerm(id: string): Promise<void> {
  return invoke<void>("delete_term", { id });
}

export async function listSubjects(termId: string): Promise<Subject[]> {
  return invoke<Subject[]>("list_subjects", { termId });
}

export async function createSubject(
  termId: string,
  input: SubjectInput,
): Promise<Subject> {
  return invoke<Subject>("create_subject", { termId, input });
}

export async function updateSubject(
  id: string,
  input: SubjectInput,
): Promise<Subject> {
  return invoke<Subject>("update_subject", { id, input });
}

export async function deleteSubject(id: string): Promise<void> {
  return invoke<void>("delete_subject", { id });
}

export async function listClassSlots(termId: string): Promise<ClassSlot[]> {
  return invoke<ClassSlot[]>("list_class_slots", { termId });
}

export async function createClassSlot(
  subjectId: string,
  input: ClassSlotInput,
): Promise<ClassSlot> {
  return invoke<ClassSlot>("create_class_slot", { subjectId, input });
}

export async function updateClassSlot(
  id: string,
  input: ClassSlotInput,
): Promise<ClassSlot> {
  return invoke<ClassSlot>("update_class_slot", { id, input });
}

export async function deleteClassSlot(id: string): Promise<void> {
  return invoke<void>("delete_class_slot", { id });
}

export async function getRotation(
  termId: string,
): Promise<RotationConfig | null> {
  return invoke<RotationConfig | null>("get_rotation", { termId });
}

export async function saveRotation(
  termId: string,
  input: RotationInput,
): Promise<RotationConfig> {
  return invoke<RotationConfig>("save_rotation", { termId, input });
}

export async function deleteRotation(termId: string): Promise<void> {
  return invoke<void>("delete_rotation", { termId });
}

export async function listHolidays(termId: string): Promise<Holiday[]> {
  return invoke<Holiday[]>("list_holidays", { termId });
}

export async function createHoliday(
  termId: string | null,
  date: string,
  name: string,
): Promise<Holiday> {
  return invoke<Holiday>("create_holiday", { termId, date, name });
}

export async function deleteHoliday(id: string): Promise<void> {
  return invoke<void>("delete_holiday", { id });
}

export async function listClassExceptions(
  termId: string,
): Promise<ClassException[]> {
  return invoke<ClassException[]>("list_class_exceptions", { termId });
}

export async function createClassException(
  slotId: string,
  date: string,
  kind: ExceptionKind,
  newStartTime: string | null,
  newEndTime: string | null,
  newRoom: string | null,
): Promise<ClassException> {
  return invoke<ClassException>("create_class_exception", {
    slotId,
    date,
    kind,
    newStartTime,
    newEndTime,
    newRoom,
  });
}

export async function deleteClassException(id: string): Promise<void> {
  return invoke<void>("delete_class_exception", { id });
}

export async function listAttendance(
  termId: string,
): Promise<AttendanceRecord[]> {
  return invoke<AttendanceRecord[]>("list_attendance", { termId });
}

export async function setAttendance(
  slotId: string,
  date: string,
  status: AttendanceStatus,
  note: string | null = null,
): Promise<AttendanceRecord> {
  return invoke<AttendanceRecord>("set_attendance", {
    slotId,
    date,
    status,
    note,
  });
}

export async function clearAttendance(
  slotId: string,
  date: string,
): Promise<void> {
  return invoke<void>("clear_attendance", { slotId, date });
}

export async function getWeekSchedule(
  termId: string,
  weekStart: string,
): Promise<WeekSchedule> {
  return invoke<WeekSchedule>("get_week_schedule", { termId, weekStart });
}

export async function getNextClass(
  termId: string,
  fromDate: string,
  fromTime: string,
): Promise<ScheduleOccurrence | null> {
  return invoke<ScheduleOccurrence | null>("get_next_class", {
    termId,
    fromDate,
    fromTime,
  });
}
