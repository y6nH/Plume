-- Your SQL goes here
CREATE TABLE blogs (
    id SERIAL PRIMARY KEY,
    actor_id VARCHAR NOT NULL,
    title VARCHAR NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    outbox_url VARCHAR NOT NULL,
    inbox_url VARCHAR NOT NULL,
    instance_id INTEGER REFERENCES instances(id) ON DELETE CASCADE NOT NULL
)
