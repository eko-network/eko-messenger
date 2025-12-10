CREATE TABLE actors (
    id SERIAL PRIMARY KEY,
    actor_url TEXT UNIQUE NOT NULL,
    is_local BOOLEAN NOT NULL,
    inbox_url TEXT NOT NULL,
    outbox_url TEXT NOT NULL,
    user_id TEXT UNIQUE NULL
);

CREATE TABLE activities (
    id SERIAL PRIMARY KEY,
    activity_id_url TEXT UNIQUE NOT NULL,
    actor_id INTEGER NOT NULL REFERENCES actors(id) ON DELETE CASCADE,
    activity_type TEXT NOT NULL,
    activity_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE inbox_entries (
    id SERIAL PRIMARY KEY,
    inbox_actor_id INTEGER NOT NULL REFERENCES actors(id) ON DELETE CASCADE,
    activity_id INTEGER NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (inbox_actor_id, activity_id)
);
