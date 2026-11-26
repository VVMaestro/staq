CREATE TABLE tasks_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'completed', 'cancelled')),
    placement TEXT NOT NULL
        CHECK (placement IN ('queue', 'stack')),
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    staq_id INTEGER REFERENCES staqs(id) ON DELETE CASCADE,
    closed_at TEXT
);

INSERT INTO tasks_new (
    id,
    name,
    description,
    status,
    placement,
    position,
    created_at,
    staq_id,
    closed_at
)
SELECT
    id,
    name,
    description,
    status,
    placement,
    position,
    created_at,
    staq_id,
    NULL
FROM tasks;

DROP TABLE tasks;

ALTER TABLE tasks_new RENAME TO tasks;

CREATE INDEX tasks_status_placement_position_idx
    ON tasks (status, placement, position);

CREATE INDEX tasks_staq_status_placement_position_idx
    ON tasks (staq_id, status, placement, position);
