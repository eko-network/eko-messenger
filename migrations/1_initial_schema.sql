CREATE TABLE refresh_tokens (
    token TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    ip_address TEXT NOT NULL,
    device_name TEXT NOT NULL,
    user_agent TEXT NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens(expires_at);
