CREATE TABLE IF NOT EXISTS accounts (
    id              INTEGER PRIMARY KEY,
    label           TEXT NOT NULL,
    group_id        INTEGER NOT NULL,
    secret          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS groups (
    id             INTEGER PRIMARY KEY,
    name           TEXT NOT NULL
);
