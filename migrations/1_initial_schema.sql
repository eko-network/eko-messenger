CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE TABLE devices (
    did SERIAL PRIMARY KEY,
    uid TEXT NOT NULL,
    name TEXT NOT NULL,
    identity_key BYTEA NOT NULL,
    registration_id INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE refresh_tokens (
		token UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    did SERIAL NOT NULL UNIQUE REFERENCES devices(did) ON DELETE CASCADE,
    ip_address TEXT NOT NULL,
    user_agent TEXT NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);


CREATE TABLE pre_keys (
    id SERIAL PRIMARY KEY,
    did SERIAL NOT NULL REFERENCES devices(did) ON DELETE CASCADE,
    key_id INTEGER NOT NULL,
    key BYTEA NOT NULL,
    UNIQUE (did, key_id)
);

CREATE TABLE signed_pre_keys (
    did SERIAL NOT NULL REFERENCES devices(did) ON DELETE CASCADE,
    key_id INTEGER NOT NULL,
    key BYTEA NOT NULL,
    signature BYTEA NOT NULL,
    PRIMARY KEY(did, key_id)
);

CREATE INDEX idx_devices_user_id ON devices(uid);
CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens(expires_at);

