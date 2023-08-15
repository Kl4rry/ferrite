
CREATE TABLE IF NOT EXISTS main.buffers(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    file_path TEXT UNIQUE NOT NULL,
    -- cursor location
    cursor_position INT,
    cursor_anchor INT,
    cursor_affinity INT
);

CREATE TABLE IF NOT EXISTS main.workspaces(
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    dir_path TEXT UNIQUE NOT NULL,
);

CREATE TABLE IF NOT EXISTS main.buffers_open(
    workspace_id INTEGER,
    FOREIGN KEY(workspace_id) REFERENCES buffers(id)
);