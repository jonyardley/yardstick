-- Phase 2 widens tasks: bucket (when) and status (state) are orthogonal
-- (handoff §Task). Added, not rebuilt: STRICT tables accept ADD COLUMN with
-- a default but not added CHECK constraints, and the typed Rust enums in
-- shared/src/task.rs are the only writers (plan decision #8).
--
-- Absent values are NULL in the database and "" / 0 across the FFI boundary
-- (see shared::TaskData). Civil dates (entered_now_on, done_on) are set by
-- the clock-free core from Event::Startup's `today`; created_at/updated_at
-- stay epoch integers set here and are never displayed.
ALTER TABLE tasks ADD COLUMN bucket         TEXT NOT NULL DEFAULT 'inbox';
ALTER TABLE tasks ADD COLUMN status         TEXT NOT NULL DEFAULT 'backlog';
ALTER TABLE tasks ADD COLUMN priority       INTEGER;
ALTER TABLE tasks ADD COLUMN due            TEXT;
ALTER TABLE tasks ADD COLUMN prev_status    TEXT;
ALTER TABLE tasks ADD COLUMN blocked_reason TEXT;
ALTER TABLE tasks ADD COLUMN source         TEXT NOT NULL DEFAULT 'quick_add';
ALTER TABLE tasks ADD COLUMN entered_now_on TEXT;
ALTER TABLE tasks ADD COLUMN done_on        TEXT;

CREATE INDEX tasks_by_bucket ON tasks(space_id, bucket) WHERE deleted_at IS NULL;
CREATE INDEX tasks_by_status ON tasks(space_id, status) WHERE deleted_at IS NULL;
