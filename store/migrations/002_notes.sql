-- One daily note per date per space.
CREATE TABLE notes (
  id          TEXT PRIMARY KEY,
  space_id    TEXT NOT NULL REFERENCES spaces(id),
  date        TEXT NOT NULL,             -- 'YYYY-MM-DD'
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  deleted_at  INTEGER,
  UNIQUE (space_id, date)
) STRICT;

-- One row per note block. Phase 1 blocks are plain paragraphs rewritten
-- wholesale on save (plan decision #3): superseded rows are hard-deleted
-- in the rewrite transaction; the note row is the tombstone unit.
-- deleted_at stays for Phase 3+ block-level editing.
CREATE TABLE blocks (
  id          TEXT PRIMARY KEY,
  space_id    TEXT NOT NULL REFERENCES spaces(id),
  note_id     TEXT NOT NULL REFERENCES notes(id),
  order_key   TEXT NOT NULL,             -- positional in P1; fractional index later
  kind        TEXT NOT NULL,             -- 'paragraph' | (later: 'heading' | 'todo' | ...)
  content     TEXT NOT NULL,             -- JSON: {"text": ...} in P1; rich spans later
  plain_text  TEXT NOT NULL,             -- extracted text; feeds `search`
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  deleted_at  INTEGER
) STRICT;
CREATE INDEX blocks_by_note ON blocks(note_id, order_key);

-- One polymorphic edge table for all refs/backlinks (note->page, task->page,
-- block->task, ...). Shipped empty in Phase 1; populated from Phase 3.
-- Deliberately NOT an entity table (no space_id/timestamps): rows are
-- identity-free edges rewritten wholesale with their source entity.
CREATE TABLE links (
  src_type TEXT NOT NULL,
  src_id   TEXT NOT NULL,
  dst_type TEXT NOT NULL,
  dst_id   TEXT NOT NULL,
  PRIMARY KEY (src_type, src_id, dst_type, dst_id)
) STRICT;
CREATE INDEX links_backlinks ON links(dst_type, dst_id);

-- Unified search index, maintained transactionally by the single writer
-- (research/persistence-fts.md §3: no triggers — all writes flow through
-- one Rust handler). FTS5 virtual tables cannot be STRICT.
CREATE VIRTUAL TABLE search USING fts5(
  entity_type UNINDEXED,   -- 'block' | 'task' | 'brief' | 'page'
  entity_id   UNINDEXED,
  title,
  body,
  tokenize = 'porter unicode61 remove_diacritics 2'
);
