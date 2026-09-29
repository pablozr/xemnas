CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    location TEXT UNIQUE NOT NULL,
    registered_at TEXT NOT NULL
);
