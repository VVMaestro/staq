CREATE TABLE staqs (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    name TEXT NOT NULL CHECK (trim(name) <> ''),
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

ALTER TABLE tasks
ADD COLUMN staq_id INTEGER REFERENCES staqs(id) ON DELETE CASCADE;

CREATE INDEX tasks_staq_status_placement_position_idx
ON tasks (staq_id, status, placement, position);
