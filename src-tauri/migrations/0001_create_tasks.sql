CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'completed')),
    placement TEXT NOT NULL
        CHECK (placement IN ('queue', 'stack')),
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX tasks_status_placement_position_idx
    ON tasks (status, placement, position);
