CREATE TABLE spaces (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  deleted_at  INTEGER
) STRICT;

INSERT INTO spaces (id, name, created_at, updated_at)
VALUES
  ('0197f000-0000-7000-8000-000000000001', 'Red Badger', unixepoch(), unixepoch()),
  ('0197f000-0000-7000-8000-000000000002', 'Yardley',    unixepoch(), unixepoch());

CREATE TABLE tasks (
  id          TEXT PRIMARY KEY,
  space_id    TEXT NOT NULL REFERENCES spaces(id),
  title       TEXT NOT NULL,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  deleted_at  INTEGER
) STRICT;

CREATE INDEX tasks_by_space ON tasks(space_id, created_at);
