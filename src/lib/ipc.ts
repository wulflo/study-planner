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
export type TaskType = "task" | "practical" | "project" | "personal";
export type TaskPriority = "low" | "medium" | "high" | "urgent";
export type TaskRecurrenceKind = "daily" | "weekly" | "monthly";

export interface TaskRecurrence {
  kind: TaskRecurrenceKind;
  interval: number;
  until: string | null;
}

export interface Tag {
  id: string;
  name: string;
  color: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface Task {
  id: string;
  subjectId: string | null;
  parentTaskId: string | null;
  taskType: TaskType;
  title: string;
  description: string | null;
  startDate: string | null;
  dueDate: string | null;
  dueTime: string | null;
  priority: TaskPriority;
  important: boolean;
  estimateMinutes: number | null;
  status: TaskStatus;
  completedAt: string | null;
  recurrence: TaskRecurrence | null;
  tags: Tag[];
  createdAt: string;
  updatedAt: string;
}

export interface TaskInput {
  subjectId: string | null;
  parentTaskId: string | null;
  taskType: TaskType;
  title: string;
  description: string | null;
  startDate: string | null;
  dueDate: string | null;
  dueTime: string | null;
  priority: number;
  important: boolean;
  estimateMinutes: number | null;
  status: TaskStatus;
  recurrence: TaskRecurrence | null;
  tagIds: string[];
}

export interface TaskFilter {
  text?: string | null;
  status?: TaskStatus | "open" | null;
  priority?: number | null;
  tagId?: string | null;
  subjectId?: string | null;
  dueFrom?: string | null;
  dueTo?: string | null;
  parentTaskId?: string | null;
  rootsOnly?: boolean;
}

export interface SavedFilter {
  id: string;
  screen: string;
  name: string;
  queryJson: string;
  createdAt: string;
  updatedAt: string;
}

export interface UndoEntry {
  id: string;
  action: string;
  entityType: string;
  entityId: string;
  beforeJson: string | null;
  afterJson: string | null;
  expiresAt: string;
  createdAt: string;
}

export type ExamResultStatus = "pending" | "passed" | "failed" | "absent";

export interface Exam {
  id: string;
  subjectId: string;
  termId: string;
  title: string;
  date: string;
  startTime: string | null;
  endTime: string | null;
  room: string | null;
  weightBp: number | null;
  resultScoreCp: number | null;
  resultMaxCp: number | null;
  resultStatus: ExamResultStatus;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface Topic {
  id: string;
  subjectId: string;
  parentTopicId: string | null;
  name: string;
  description: string | null;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface ExamTopic {
  id: string;
  examId: string;
  topicId: string;
  createdAt: string;
  updatedAt: string;
}

export interface ExamInput {
  title: string;
  date: string;
  startTime: string | null;
  endTime: string | null;
  room: string | null;
  weightBp: number | null;
  resultScoreCp: number | null;
  resultMaxCp: number | null;
  resultStatus: ExamResultStatus;
  notes: string | null;
}

export interface TopicInput {
  parentTopicId: string | null;
  name: string;
  description: string | null;
  sortOrder: number;
}

export interface GradeScale {
  id: string;
  subjectId: string | null;
  name: string;
  minCp: number;
  maxCp: number;
  passCp: number;
  precision: number;
  isDefault: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface GradeCategory {
  id: string;
  subjectId: string;
  scaleId: string;
  name: string;
  weightBp: number;
  dropLowest: number;
  createdAt: string;
  updatedAt: string;
}

export interface GradeEntry {
  id: string;
  subjectId: string;
  categoryId: string;
  examId: string | null;
  title: string;
  scoreCp: number;
  maxCp: number;
  weightBp: number | null;
  date: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CategoryGrade {
  categoryId: string;
  categoryName: string;
  weightBp: number;
  entries: number;
  dropped: number;
  percentBp: number | null;
  averageCp: number | null;
}

export interface SubjectGrade {
  subjectId: string;
  scaleId: string | null;
  finalBp: number | null;
  finalCp: number | null;
  passed: boolean | null;
  categories: CategoryGrade[];
}

export interface GradeScaleInput {
  subjectId: string | null;
  name: string;
  minCp: number;
  maxCp: number;
  passCp: number;
  precision: number;
  isDefault: boolean;
}

export interface GradeCategoryInput {
  scaleId: string;
  name: string;
  weightBp: number;
  dropLowest: number;
}

export interface GradeEntryInput {
  categoryId: string;
  examId: string | null;
  title: string;
  scoreCp: number;
  maxCp: number;
  weightBp: number | null;
  date: string | null;
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

export async function createTaskFull(input: TaskInput): Promise<Task> {
  return invoke<Task>("create_task_full", { input });
}

export async function updateTask(id: string, input: TaskInput): Promise<Task> {
  return invoke<Task>("update_task", { id, input });
}

export async function setTaskStatus(
  id: string,
  status: TaskStatus,
): Promise<Task> {
  return invoke<Task>("set_task_status", { id, status });
}

export async function searchTasks(filter: TaskFilter): Promise<Task[]> {
  return invoke<Task[]>("search_tasks", { filter });
}

export async function listTags(): Promise<Tag[]> {
  return invoke<Tag[]>("list_tags");
}

export async function createTag(
  name: string,
  color: string | null = null,
): Promise<Tag> {
  return invoke<Tag>("create_tag", { name, color });
}

export async function updateTag(
  id: string,
  name: string,
  color: string | null = null,
): Promise<Tag> {
  return invoke<Tag>("update_tag", { id, name, color });
}

export async function deleteTag(id: string): Promise<void> {
  return invoke<void>("delete_tag", { id });
}

export async function setTaskTags(
  taskId: string,
  tagIds: string[],
): Promise<void> {
  return invoke<void>("set_task_tags", { taskId, tagIds });
}

export async function listSavedFilters(
  screen: string | null = null,
): Promise<SavedFilter[]> {
  return invoke<SavedFilter[]>("list_saved_filters", { screen });
}

export async function createSavedFilter(
  screen: string,
  name: string,
  queryJson: string,
): Promise<SavedFilter> {
  return invoke<SavedFilter>("create_saved_filter", {
    screen,
    name,
    queryJson,
  });
}

export async function updateSavedFilter(
  id: string,
  name: string,
  queryJson: string,
): Promise<SavedFilter> {
  return invoke<SavedFilter>("update_saved_filter", {
    id,
    name,
    queryJson,
  });
}

export async function deleteSavedFilter(id: string): Promise<void> {
  return invoke<void>("delete_saved_filter", { id });
}

export async function listUndoEntries(): Promise<UndoEntry[]> {
  return invoke<UndoEntry[]>("list_undo_entries");
}

export async function undoEntry(id: string): Promise<Task | null> {
  return invoke<Task | null>("undo_entry", { id });
}

export async function purgeExpiredUndo(): Promise<number> {
  return invoke<number>("purge_expired_undo");
}

// ---------------------------------------------------------------------------
// Exámenes y temas
// ---------------------------------------------------------------------------

export async function listExams(
  termId: string | null,
  subjectId: string | null = null,
): Promise<Exam[]> {
  return invoke<Exam[]>("list_exams", { termId, subjectId });
}

export async function createExam(
  subjectId: string,
  termId: string,
  input: ExamInput,
): Promise<Exam> {
  return invoke<Exam>("create_exam", { subjectId, termId, input });
}

export async function updateExam(id: string, input: ExamInput): Promise<Exam> {
  return invoke<Exam>("update_exam", { id, input });
}

export async function deleteExam(id: string): Promise<void> {
  return invoke<void>("delete_exam", { id });
}

export async function listTopics(subjectId: string): Promise<Topic[]> {
  return invoke<Topic[]>("list_topics", { subjectId });
}

export async function createTopic(
  subjectId: string,
  input: TopicInput,
): Promise<Topic> {
  return invoke<Topic>("create_topic", { subjectId, input });
}

export async function updateTopic(
  id: string,
  input: TopicInput,
): Promise<Topic> {
  return invoke<Topic>("update_topic", { id, input });
}

export async function deleteTopic(id: string): Promise<void> {
  return invoke<void>("delete_topic", { id });
}

export async function listExamTopics(examId: string): Promise<ExamTopic[]> {
  return invoke<ExamTopic[]>("list_exam_topics", { examId });
}

export async function linkExamTopic(
  examId: string,
  topicId: string,
): Promise<ExamTopic> {
  return invoke<ExamTopic>("link_exam_topic", { examId, topicId });
}

export async function unlinkExamTopic(
  examId: string,
  topicId: string,
): Promise<void> {
  return invoke<void>("unlink_exam_topic", { examId, topicId });
}

// ---------------------------------------------------------------------------
// Notas
// ---------------------------------------------------------------------------

export async function listGradeScales(
  subjectId: string | null,
): Promise<GradeScale[]> {
  return invoke<GradeScale[]>("list_grade_scales", { subjectId });
}

export async function createGradeScale(
  input: GradeScaleInput,
): Promise<GradeScale> {
  return invoke<GradeScale>("create_grade_scale", { input });
}

export async function updateGradeScale(
  id: string,
  input: GradeScaleInput,
): Promise<GradeScale> {
  return invoke<GradeScale>("update_grade_scale", { id, input });
}

export async function deleteGradeScale(id: string): Promise<void> {
  return invoke<void>("delete_grade_scale", { id });
}

export async function listGradeCategories(
  subjectId: string,
): Promise<GradeCategory[]> {
  return invoke<GradeCategory[]>("list_grade_categories", { subjectId });
}

export async function createGradeCategory(
  subjectId: string,
  input: GradeCategoryInput,
): Promise<GradeCategory> {
  return invoke<GradeCategory>("create_grade_category", { subjectId, input });
}

export async function updateGradeCategory(
  id: string,
  input: GradeCategoryInput,
): Promise<GradeCategory> {
  return invoke<GradeCategory>("update_grade_category", { id, input });
}

export async function deleteGradeCategory(id: string): Promise<void> {
  return invoke<void>("delete_grade_category", { id });
}

export async function listGradeEntries(
  subjectId: string,
): Promise<GradeEntry[]> {
  return invoke<GradeEntry[]>("list_grade_entries", { subjectId });
}

export async function createGradeEntry(
  subjectId: string,
  input: GradeEntryInput,
): Promise<GradeEntry> {
  return invoke<GradeEntry>("create_grade_entry", { subjectId, input });
}

export async function updateGradeEntry(
  id: string,
  input: GradeEntryInput,
): Promise<GradeEntry> {
  return invoke<GradeEntry>("update_grade_entry", { id, input });
}

export async function deleteGradeEntry(id: string): Promise<void> {
  return invoke<void>("delete_grade_entry", { id });
}

export async function getSubjectGrade(
  subjectId: string,
): Promise<SubjectGrade> {
  return invoke<SubjectGrade>("get_subject_grade", { subjectId });
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

// ---------------------------------------------------------------------------
// Estudio, autoevaluación, repaso y estadísticas
// ---------------------------------------------------------------------------

export type AssessmentSource = "manual" | "prompt" | "review";
export type StrengthClassification = "weak" | "neutral" | "strength";
export type StudySessionMode = "pomodoro" | "free" | "review";

export interface SelfAssessment {
  id: string;
  topicId: string;
  score: number;
  note: string | null;
  source: AssessmentSource;
  assessedAt: string;
  createdAt: string;
  updatedAt: string;
}

export interface StrengthOverride {
  id: string;
  topicId: string;
  classification: StrengthClassification;
  reason: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ReviewItem {
  id: string;
  topicId: string;
  algorithm: string;
  algorithmVersion: string;
  easeFactorCp: number;
  intervalDays: number;
  repetitions: number;
  lapses: number;
  dueDate: string | null;
  lastReviewedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface StudySession {
  id: string;
  subjectId: string | null;
  topicId: string | null;
  startedAt: string;
  endedAt: string | null;
  plannedMinutes: number | null;
  actualMinutes: number | null;
  mode: StudySessionMode;
  interruptions: number;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface TopicStudy {
  topicId: string;
  subjectId: string;
  parentTopicId: string | null;
  name: string;
  description: string | null;
  sortOrder: number;
  latestAssessment: SelfAssessment | null;
  derivedClassification: StrengthClassification;
  overrideClassification: StrengthClassification | null;
  classification: StrengthClassification;
  overrideReason: string | null;
  reviewItem: ReviewItem | null;
  nextExamDate: string | null;
}

export interface PriorityReason {
  code: string;
  label: string;
  points: number;
}

export interface StudyPriorityView {
  topicId: string;
  topicName: string;
  subjectId: string;
  subjectName: string;
  score: number;
  classification: StrengthClassification;
  reasons: PriorityReason[];
  reviewDue: string | null;
  nextExamDate: string | null;
}

export interface SubjectMinutesView {
  subjectId: string | null;
  subjectName: string | null;
  minutes: number;
  sessions: number;
}

export interface WeakSubjectView {
  subjectId: string;
  subjectName: string;
  topicCount: number;
  weakTopics: number;
  averageScoreBp: number | null;
}

export interface UpcomingReviewView {
  topicId: string;
  topicName: string;
  subjectId: string;
  subjectName: string;
  dueDate: string;
  daysUntil: number;
}

export interface StudyTotals {
  totalMinutes: number;
  sessionCount: number;
  currentStreakDays: number;
  longestStreakDays: number;
}

export interface StudyStats {
  totals: StudyTotals;
  minutesBySubject: SubjectMinutesView[];
  weakSubjects: WeakSubjectView[];
  upcomingReviews: UpcomingReviewView[];
  dueReviews: number;
}

export interface UpdaterStatus {
  publicKeyConfigured: boolean;
  currentVersion: string;
  updateHost: string;
  autoCheck: boolean;
  runningAsAppImage: boolean;
  canRollback: boolean;
}

export interface UpdateInfo {
  version: string;
  pubDate: string | null;
  notes: string;
}

export interface InstallOutcome {
  version: string;
  restartRequired: boolean;
}

export interface NetworkEvent {
  id: string;
  occurredAt: string;
  kind: string;
  host: string;
  bytesSent: number;
  bytesReceived: number;
  outcome: string;
  detail: string | null;
  createdAt: string;
  updatedAt: string;
}

export async function listSelfAssessments(
  topicId: string,
): Promise<SelfAssessment[]> {
  return invoke<SelfAssessment[]>("list_self_assessments", { topicId });
}

export async function createSelfAssessment(
  topicId: string,
  score: number,
  note: string | null = null,
  source: AssessmentSource = "manual",
): Promise<SelfAssessment> {
  return invoke<SelfAssessment>("create_self_assessment", {
    topicId,
    score,
    note,
    source,
  });
}

export async function deleteSelfAssessment(id: string): Promise<void> {
  return invoke<void>("delete_self_assessment", { id });
}

export async function listStrengthOverrides(): Promise<StrengthOverride[]> {
  return invoke<StrengthOverride[]>("list_strength_overrides");
}

export async function setStrengthOverride(
  topicId: string,
  classification: StrengthClassification,
  reason: string | null = null,
): Promise<StrengthOverride> {
  return invoke<StrengthOverride>("set_strength_override", {
    topicId,
    classification,
    reason,
  });
}

export async function clearStrengthOverride(topicId: string): Promise<void> {
  return invoke<void>("clear_strength_override", { topicId });
}

export async function listReviewItems(
  subjectId: string | null = null,
): Promise<ReviewItem[]> {
  return invoke<ReviewItem[]>("list_review_items", { subjectId });
}

export async function reviewTopic(
  topicId: string,
  quality: number,
  today: string,
): Promise<ReviewItem> {
  return invoke<ReviewItem>("review_topic", { topicId, quality, today });
}

export async function setReviewDue(
  topicId: string,
  dueDate: string,
): Promise<ReviewItem> {
  return invoke<ReviewItem>("set_review_due", { topicId, dueDate });
}

export async function deleteReviewItem(topicId: string): Promise<void> {
  return invoke<void>("delete_review_item", { topicId });
}

export async function listStudySessions(
  from: string | null = null,
  to: string | null = null,
  subjectId: string | null = null,
): Promise<StudySession[]> {
  return invoke<StudySession[]>("list_study_sessions", {
    from,
    to,
    subjectId,
  });
}

export async function startStudySession(
  subjectId: string | null,
  topicId: string | null,
  mode: StudySessionMode,
  plannedMinutes: number | null = null,
): Promise<StudySession> {
  return invoke<StudySession>("start_study_session", {
    subjectId,
    topicId,
    mode,
    plannedMinutes,
  });
}

export async function finishStudySession(
  id: string,
  actualMinutes: number | null,
  interruptions = 0,
  notes: string | null = null,
): Promise<StudySession> {
  return invoke<StudySession>("finish_study_session", {
    id,
    actualMinutes,
    interruptions,
    notes,
  });
}

export async function deleteStudySession(id: string): Promise<void> {
  return invoke<void>("delete_study_session", { id });
}

export async function listTopicStudies(
  subjectId: string | null,
  today: string,
): Promise<TopicStudy[]> {
  return invoke<TopicStudy[]>("list_topic_studies", { subjectId, today });
}

export async function listStudyPriorities(
  today: string,
): Promise<StudyPriorityView[]> {
  return invoke<StudyPriorityView[]>("list_study_priorities", { today });
}

export async function getStudyStats(
  from: string | null,
  today: string,
): Promise<StudyStats> {
  return invoke<StudyStats>("get_study_stats", { from, today });
}

export async function getUpdaterStatus(): Promise<UpdaterStatus> {
  return invoke<UpdaterStatus>("get_updater_status");
}

export async function setUpdaterAutoCheck(
  enabled: boolean,
): Promise<UpdaterStatus> {
  return invoke<UpdaterStatus>("set_updater_auto_check", { enabled });
}

/** Devuelve `null` cuando la versión instalada ya es la más nueva. */
export async function checkUpdates(): Promise<UpdateInfo | null> {
  return invoke<UpdateInfo | null>("check_updates");
}

/** Descarga, verifica e instala; vuelve a consultar el manifiesto en Rust. */
export async function installUpdate(): Promise<InstallOutcome> {
  return invoke<InstallOutcome>("install_update");
}

export async function rollbackUpdate(): Promise<void> {
  return invoke<void>("rollback_update");
}

export async function listNetworkEvents(): Promise<NetworkEvent[]> {
  return invoke<NetworkEvent[]>("list_network_events");
}
