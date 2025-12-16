CREATE TABLE actors (
    id TEXT PRIMARY KEY,
    is_local BOOLEAN NOT NULL,
    inbox_url TEXT NOT NULL,
    outbox_url TEXT NOT NULL
);

CREATE TABLE activities (
    id TEXT PRIMARY KEY,
    actor_id TEXT NOT NULL REFERENCES actors(id) ON DELETE CASCADE,
    activity_type TEXT NOT NULL,
    activity_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE inbox_entries (
    id SERIAL PRIMARY KEY,
    inbox_actor_id TEXT NOT NULL REFERENCES actors(id) ON DELETE CASCADE,
    activity_id TEXT NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
    UNIQUE (inbox_actor_id, activity_id)
);
