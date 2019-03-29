-- Your SQL goes here
CREATE TABLE api_tokens (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    creation_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    value TEXT NOT NULL,
    scopes TEXT NOT NULL,
    app_id INTEGER NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE
)
