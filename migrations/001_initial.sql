CREATE TABLE IF NOT EXISTS monitors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    check_interval_secs INTEGER NOT NULL DEFAULT 60 CHECK(check_interval_secs >= 1),
    timeout_secs INTEGER NOT NULL DEFAULT 30 CHECK(timeout_secs >= 1),
    status TEXT NOT NULL DEFAULT 'unknown' CHECK(status IN ('up', 'down', 'unknown')),
    last_check_at TEXT,
    last_response_time_ms INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_monitors_last_check_at ON monitors(last_check_at);
CREATE INDEX IF NOT EXISTS idx_monitors_status ON monitors(status);

CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    monitor_id INTEGER NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('up', 'down')),
    response_time_ms INTEGER,
    error_message TEXT,
    checked_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (monitor_id) REFERENCES monitors(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_history_monitor_id ON history(monitor_id, id DESC);
