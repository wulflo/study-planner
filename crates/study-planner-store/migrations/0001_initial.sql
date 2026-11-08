-- Study Planner schema v1
-- Convenciones:
--   * id: UUIDv7 en TEXT.
--   * *_at: UTC en RFC3339 con milisegundos.
--   * fechas académicas: YYYY-MM-DD (fecha civil).
--   * horas locales: HH:MM.
--   * puntajes y pesos: enteros de punto fijo (centipuntos / basis points).
--   * deleted_at distinto de NULL = borrado lógico.
--   * No se permiten borrados físicos de entidades de dominio.

CREATE TABLE app_meta (
  key        TEXT PRIMARY KEY,
  value      TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE TABLE schema_migration (
  version    INTEGER PRIMARY KEY,
  name       TEXT NOT NULL,
  checksum   TEXT NOT NULL,
  applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE TABLE academic_year (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  start_date TEXT NOT NULL,
  end_date   TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at TEXT,
  CHECK (start_date <= end_date)
) STRICT;

CREATE TABLE rotation_config (
  id              TEXT PRIMARY KEY,
  term_id         TEXT NOT NULL REFERENCES term(id) ON DELETE RESTRICT,
  mode            TEXT NOT NULL CHECK (mode IN ('weekly_ab', 'parity', 'day_cycle')),
  anchor_date     TEXT NOT NULL,
  cycle_length    INTEGER NOT NULL DEFAULT 2 CHECK (cycle_length >= 1),
  labels_json     TEXT NOT NULL DEFAULT '[]',
  day_cycle_json  TEXT NOT NULL DEFAULT '[]',
  created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at      TEXT
) STRICT;

CREATE TABLE term (
  id          TEXT PRIMARY KEY,
  year_id     TEXT NOT NULL REFERENCES academic_year(id) ON DELETE RESTRICT,
  name        TEXT NOT NULL,
  kind        TEXT NOT NULL DEFAULT 'semester'
                CHECK (kind IN ('semester', 'trimester', 'quarter', 'bimonth', 'annual', 'other')),
  start_date  TEXT NOT NULL,
  end_date    TEXT NOT NULL,
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at  TEXT,
  CHECK (start_date <= end_date)
) STRICT;

CREATE TABLE holiday (
  id         TEXT PRIMARY KEY,
  term_id    TEXT REFERENCES term(id) ON DELETE RESTRICT,
  date       TEXT NOT NULL,
  name       TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at TEXT
) STRICT;

CREATE TABLE subject (
  id                TEXT PRIMARY KEY,
  term_id           TEXT NOT NULL REFERENCES term(id) ON DELETE RESTRICT,
  name              TEXT NOT NULL,
  code              TEXT,
  color             TEXT NOT NULL DEFAULT '#6750A4',
  teacher           TEXT,
  room              TEXT,
  credits           INTEGER CHECK (credits IS NULL OR credits >= 0),
  contact           TEXT,
  online_url        TEXT,
  goal_score_cp     INTEGER CHECK (goal_score_cp IS NULL OR goal_score_cp >= 0),
  attendance_min_bp INTEGER CHECK (attendance_min_bp IS NULL OR attendance_min_bp BETWEEN 0 AND 10000),
  created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at        TEXT
) STRICT;

CREATE TABLE class_slot (
  id            TEXT PRIMARY KEY,
  subject_id    TEXT NOT NULL REFERENCES subject(id) ON DELETE RESTRICT,
  weekday       INTEGER NOT NULL CHECK (weekday BETWEEN 1 AND 7),
  start_time    TEXT NOT NULL,
  end_time      TEXT NOT NULL,
  recurrence    TEXT NOT NULL DEFAULT 'weekly' CHECK (recurrence IN ('weekly', 'biweekly')),
  rotation_key  TEXT,
  valid_from    TEXT,
  valid_to      TEXT,
  room_override TEXT,
  created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at    TEXT,
  CHECK (start_time < end_time)
) STRICT;

CREATE TABLE class_exception (
  id            TEXT PRIMARY KEY,
  class_slot_id TEXT NOT NULL REFERENCES class_slot(id) ON DELETE RESTRICT,
  date          TEXT NOT NULL,
  kind          TEXT NOT NULL CHECK (kind IN ('cancelled', 'moved', 'extra')),
  new_start_time TEXT,
  new_end_time   TEXT,
  new_room       TEXT,
  created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at    TEXT
) STRICT;

CREATE UNIQUE INDEX class_exception_unique
  ON class_exception(class_slot_id, date, kind)
  WHERE deleted_at IS NULL;

CREATE TABLE attendance_record (
  id            TEXT PRIMARY KEY,
  class_slot_id TEXT NOT NULL REFERENCES class_slot(id) ON DELETE RESTRICT,
  date          TEXT NOT NULL,
  status        TEXT NOT NULL CHECK (status IN ('present', 'absent', 'late', 'justified')),
  note          TEXT,
  created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at    TEXT
) STRICT;

CREATE UNIQUE INDEX attendance_unique
  ON attendance_record(class_slot_id, date)
  WHERE deleted_at IS NULL;

CREATE TABLE exam (
  id              TEXT PRIMARY KEY,
  subject_id      TEXT NOT NULL REFERENCES subject(id) ON DELETE RESTRICT,
  term_id         TEXT NOT NULL REFERENCES term(id) ON DELETE RESTRICT,
  title           TEXT NOT NULL,
  date            TEXT NOT NULL,
  start_time      TEXT,
  end_time        TEXT,
  room            TEXT,
  weight_bp       INTEGER CHECK (weight_bp IS NULL OR weight_bp BETWEEN 0 AND 10000),
  result_score_cp INTEGER,
  result_max_cp   INTEGER,
  result_status   TEXT NOT NULL DEFAULT 'pending'
                    CHECK (result_status IN ('pending', 'passed', 'failed', 'absent')),
  notes           TEXT,
  created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at      TEXT
) STRICT;

CREATE TABLE topic (
  id              TEXT PRIMARY KEY,
  subject_id      TEXT NOT NULL REFERENCES subject(id) ON DELETE RESTRICT,
  parent_topic_id TEXT REFERENCES topic(id) ON DELETE RESTRICT,
  name            TEXT NOT NULL,
  description     TEXT,
  sort_order      INTEGER NOT NULL DEFAULT 0,
  created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at      TEXT
) STRICT;

CREATE TABLE exam_topic (
  id         TEXT PRIMARY KEY,
  exam_id    TEXT NOT NULL REFERENCES exam(id) ON DELETE RESTRICT,
  topic_id   TEXT NOT NULL REFERENCES topic(id) ON DELETE RESTRICT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at TEXT
) STRICT;

CREATE UNIQUE INDEX exam_topic_unique
  ON exam_topic(exam_id, topic_id)
  WHERE deleted_at IS NULL;

CREATE TABLE grade_scale (
  id         TEXT PRIMARY KEY,
  subject_id TEXT REFERENCES subject(id) ON DELETE RESTRICT,
  name       TEXT NOT NULL,
  min_cp     INTEGER NOT NULL,
  max_cp     INTEGER NOT NULL,
  pass_cp    INTEGER NOT NULL,
  precision  INTEGER NOT NULL DEFAULT 2 CHECK (precision BETWEEN 0 AND 4),
  is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at TEXT,
  CHECK (min_cp < max_cp),
  CHECK (pass_cp BETWEEN min_cp AND max_cp)
) STRICT;

CREATE TABLE grade_category (
  id          TEXT PRIMARY KEY,
  subject_id  TEXT NOT NULL REFERENCES subject(id) ON DELETE RESTRICT,
  scale_id    TEXT NOT NULL REFERENCES grade_scale(id) ON DELETE RESTRICT,
  name        TEXT NOT NULL,
  weight_bp   INTEGER NOT NULL CHECK (weight_bp BETWEEN 0 AND 10000),
  drop_lowest INTEGER NOT NULL DEFAULT 0 CHECK (drop_lowest >= 0),
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at  TEXT
) STRICT;

CREATE TABLE grade_entry (
  id          TEXT PRIMARY KEY,
  subject_id  TEXT NOT NULL REFERENCES subject(id) ON DELETE RESTRICT,
  category_id TEXT NOT NULL REFERENCES grade_category(id) ON DELETE RESTRICT,
  exam_id     TEXT REFERENCES exam(id) ON DELETE RESTRICT,
  title       TEXT NOT NULL,
  score_cp    INTEGER NOT NULL,
  max_cp      INTEGER NOT NULL,
  weight_bp   INTEGER CHECK (weight_bp IS NULL OR weight_bp BETWEEN 0 AND 10000),
  date        TEXT,
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at  TEXT,
  CHECK (max_cp > 0),
  CHECK (score_cp BETWEEN 0 AND max_cp)
) STRICT;

CREATE TABLE task (
  id               TEXT PRIMARY KEY,
  subject_id       TEXT REFERENCES subject(id) ON DELETE RESTRICT,
  parent_task_id   TEXT REFERENCES task(id) ON DELETE RESTRICT,
  type             TEXT NOT NULL DEFAULT 'task'
                     CHECK (type IN ('task', 'practical', 'project', 'personal')),
  title            TEXT NOT NULL,
  description      TEXT,
  start_date       TEXT,
  due_date         TEXT,
  due_time         TEXT,
  priority         INTEGER NOT NULL DEFAULT 0 CHECK (priority BETWEEN 0 AND 3),
  important        INTEGER NOT NULL DEFAULT 0 CHECK (important IN (0, 1)),
  estimate_minutes INTEGER CHECK (estimate_minutes IS NULL OR estimate_minutes >= 0),
  status           TEXT NOT NULL DEFAULT 'pending'
                     CHECK (status IN ('pending', 'in_progress', 'done')),
  completed_at     TEXT,
  recurrence_json  TEXT,
  created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at       TEXT,
  CHECK (parent_task_id IS NULL OR parent_task_id <> id),
  CHECK (due_date IS NULL OR start_date IS NULL OR start_date <= due_date)
) STRICT;

CREATE TABLE tag (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  color      TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at TEXT
) STRICT;

CREATE UNIQUE INDEX tag_name_unique
  ON tag(name)
  WHERE deleted_at IS NULL;

CREATE TABLE task_tag (
  id         TEXT PRIMARY KEY,
  task_id    TEXT NOT NULL REFERENCES task(id) ON DELETE RESTRICT,
  tag_id     TEXT NOT NULL REFERENCES tag(id) ON DELETE RESTRICT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at TEXT
) STRICT;

CREATE UNIQUE INDEX task_tag_unique
  ON task_tag(task_id, tag_id)
  WHERE deleted_at IS NULL;

CREATE TABLE saved_filter (
  id         TEXT PRIMARY KEY,
  screen     TEXT NOT NULL,
  name       TEXT NOT NULL,
  query_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at TEXT
) STRICT;

CREATE TABLE self_assessment (
  id          TEXT PRIMARY KEY,
  topic_id    TEXT NOT NULL REFERENCES topic(id) ON DELETE RESTRICT,
  score       INTEGER NOT NULL CHECK (score BETWEEN 1 AND 5),
  note        TEXT,
  source      TEXT NOT NULL DEFAULT 'manual' CHECK (source IN ('manual', 'prompt', 'review')),
  assessed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at  TEXT
) STRICT;

CREATE TABLE strength_override (
  id             TEXT PRIMARY KEY,
  topic_id       TEXT NOT NULL UNIQUE REFERENCES topic(id) ON DELETE RESTRICT,
  classification TEXT NOT NULL CHECK (classification IN ('strength', 'neutral', 'weak')),
  reason         TEXT,
  created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at     TEXT
) STRICT;

CREATE TABLE review_item (
  id               TEXT PRIMARY KEY,
  topic_id         TEXT NOT NULL UNIQUE REFERENCES topic(id) ON DELETE RESTRICT,
  algorithm        TEXT NOT NULL DEFAULT 'sm2' CHECK (algorithm IN ('sm2')),
  algorithm_version TEXT NOT NULL DEFAULT '1',
  ease_factor_cp   INTEGER NOT NULL DEFAULT 250 CHECK (ease_factor_cp >= 130),
  interval_days    INTEGER NOT NULL DEFAULT 0 CHECK (interval_days >= 0),
  repetitions      INTEGER NOT NULL DEFAULT 0 CHECK (repetitions >= 0),
  lapses           INTEGER NOT NULL DEFAULT 0 CHECK (lapses >= 0),
  due_date         TEXT,
  last_reviewed_at TEXT,
  created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at       TEXT
) STRICT;

CREATE TABLE study_session (
  id             TEXT PRIMARY KEY,
  subject_id     TEXT REFERENCES subject(id) ON DELETE RESTRICT,
  topic_id       TEXT REFERENCES topic(id) ON DELETE RESTRICT,
  started_at     TEXT NOT NULL,
  ended_at       TEXT,
  planned_minutes INTEGER CHECK (planned_minutes IS NULL OR planned_minutes >= 0),
  actual_minutes  INTEGER CHECK (actual_minutes IS NULL OR actual_minutes >= 0),
  mode           TEXT NOT NULL DEFAULT 'pomodoro' CHECK (mode IN ('pomodoro', 'free', 'review')),
  interruptions  INTEGER NOT NULL DEFAULT 0 CHECK (interruptions >= 0),
  notes          TEXT,
  created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at     TEXT,
  CHECK (ended_at IS NULL OR ended_at >= started_at)
) STRICT;

CREATE TABLE note (
  id         TEXT PRIMARY KEY,
  subject_id TEXT REFERENCES subject(id) ON DELETE RESTRICT,
  topic_id   TEXT REFERENCES topic(id) ON DELETE RESTRICT,
  title      TEXT NOT NULL,
  body       TEXT NOT NULL DEFAULT '',
  pinned     INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at TEXT
) STRICT;

CREATE TABLE attachment (
  id         TEXT PRIMARY KEY,
  owner_type TEXT NOT NULL CHECK (owner_type IN ('subject', 'exam', 'task', 'topic', 'note')),
  owner_id   TEXT NOT NULL,
  filename   TEXT NOT NULL,
  mime       TEXT NOT NULL,
  size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
  sha256     TEXT NOT NULL,
  storage    TEXT NOT NULL DEFAULT 'vault' CHECK (storage IN ('vault', 'embedded')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at TEXT
) STRICT;

CREATE UNIQUE INDEX attachment_owner_name_unique
  ON attachment(owner_type, owner_id, filename)
  WHERE deleted_at IS NULL;

CREATE INDEX attachment_sha256_idx ON attachment(sha256);

CREATE TABLE notification (
  id           TEXT PRIMARY KEY,
  kind         TEXT NOT NULL CHECK (kind IN ('class', 'task', 'exam', 'review', 'backup', 'update')),
  entity_type  TEXT,
  entity_id    TEXT,
  scheduled_at TEXT NOT NULL,
  fired_at     TEXT,
  snooze_until TEXT,
  state        TEXT NOT NULL DEFAULT 'scheduled'
                 CHECK (state IN ('scheduled', 'fired', 'snoozed', 'dismissed', 'done')),
  dedupe_key   TEXT NOT NULL UNIQUE,
  created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at   TEXT
) STRICT;

CREATE TABLE network_event (
  id             TEXT PRIMARY KEY,
  occurred_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  kind           TEXT NOT NULL,
  host           TEXT NOT NULL,
  bytes_sent     INTEGER NOT NULL DEFAULT 0 CHECK (bytes_sent >= 0),
  bytes_received INTEGER NOT NULL DEFAULT 0 CHECK (bytes_received >= 0),
  outcome        TEXT NOT NULL,
  detail         TEXT,
  created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at     TEXT
) STRICT;

CREATE TABLE undo_journal (
  id          TEXT PRIMARY KEY,
  action      TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id   TEXT NOT NULL,
  before_json TEXT,
  after_json  TEXT,
  expires_at  TEXT NOT NULL,
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  deleted_at  TEXT
) STRICT;

CREATE TABLE setting (
  key        TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  scope      TEXT NOT NULL DEFAULT 'portable' CHECK (scope IN ('portable', 'machine')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

-- Búsqueda local. Las tablas FTS son derivadas y se reconstruyen al importar.
CREATE VIRTUAL TABLE task_fts USING fts5(
  task_id UNINDEXED,
  title,
  description,
  tags,
  tokenize = 'unicode61'
);

CREATE VIRTUAL TABLE note_fts USING fts5(
  note_id UNINDEXED,
  title,
  body,
  tokenize = 'unicode61'
);

CREATE INDEX academic_year_dates_idx ON academic_year(start_date, end_date);
CREATE INDEX holiday_date_idx ON holiday(date);
CREATE INDEX subject_term_idx ON subject(term_id) WHERE deleted_at IS NULL;

CREATE INDEX class_slot_subject_idx ON class_slot(subject_id) WHERE deleted_at IS NULL;
CREATE INDEX class_slot_weekday_idx ON class_slot(weekday) WHERE deleted_at IS NULL;
CREATE INDEX class_exception_date_idx ON class_exception(date) WHERE deleted_at IS NULL;

CREATE INDEX exam_subject_date_idx ON exam(subject_id, date) WHERE deleted_at IS NULL;
CREATE INDEX exam_date_idx ON exam(date) WHERE deleted_at IS NULL;

CREATE INDEX topic_subject_idx ON topic(subject_id) WHERE deleted_at IS NULL;
CREATE INDEX topic_parent_idx ON topic(parent_topic_id) WHERE deleted_at IS NULL;

CREATE INDEX self_assessment_topic_idx
  ON self_assessment(topic_id, assessed_at)
  WHERE deleted_at IS NULL;

CREATE INDEX review_item_due_idx
  ON review_item(due_date)
  WHERE deleted_at IS NULL;

CREATE INDEX grade_category_subject_idx
  ON grade_category(subject_id)
  WHERE deleted_at IS NULL;

CREATE INDEX grade_entry_subject_date_idx
  ON grade_entry(subject_id, date)
  WHERE deleted_at IS NULL;

CREATE INDEX task_due_idx ON task(due_date, status) WHERE deleted_at IS NULL;
CREATE INDEX task_parent_idx ON task(parent_task_id) WHERE deleted_at IS NULL;
CREATE INDEX task_subject_idx ON task(subject_id) WHERE deleted_at IS NULL;
CREATE INDEX task_status_idx ON task(status) WHERE deleted_at IS NULL;

CREATE INDEX saved_filter_screen_idx ON saved_filter(screen) WHERE deleted_at IS NULL;

CREATE INDEX study_session_started_idx
  ON study_session(started_at)
  WHERE deleted_at IS NULL;

CREATE INDEX note_subject_idx ON note(subject_id) WHERE deleted_at IS NULL;

CREATE INDEX notification_scheduled_idx
  ON notification(scheduled_at, state)
  WHERE deleted_at IS NULL;

CREATE INDEX network_event_occurred_idx ON network_event(occurred_at);

CREATE INDEX undo_journal_expires_idx ON undo_journal(expires_at);

-- Ajustes iniciales. Los valores machine no se exportan ni se importan.
INSERT INTO setting(key, value_json, scope) VALUES
  ('theme.mode', '"system"', 'portable'),
  ('theme.accent', '"#6750A4"', 'portable'),
  ('theme.variant', '"tonal_spot"', 'portable'),
  ('theme.contrast', '"standard"', 'portable'),
  ('locale', '"es"', 'portable'),
  ('backup.retention.daily', '7', 'portable'),
  ('backup.retention.weekly', '4', 'portable'),
  ('backup.retention.monthly', '6', 'portable'),
  ('backup.interval_hours', '6', 'portable'),
  ('notifications.enabled', 'true', 'portable'),
  ('notifications.class_lead_minutes', '10', 'portable'),
  ('notifications.task_lead_minutes', '60', 'portable'),
  ('notifications.exam_lead_hours', '24', 'portable'),
  ('study.weak_threshold', '0.4', 'portable'),
  ('study.strong_threshold', '0.7', 'portable'),
  ('study.priority_decay_days', '7', 'portable'),
  ('updater.auto_check', 'false', 'portable'),
  ('updater.strict_offline', 'false', 'portable'),
  ('paths.vault', 'null', 'machine'),
  ('paths.backups', 'null', 'machine'),
  ('locale.timezone', 'null', 'machine'),
  ('window.last_route', 'null', 'machine');
