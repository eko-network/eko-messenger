CREATE TABLE IF NOT EXISTS encrypted_group_states (
    id TEXT PRIMARY KEY,

    -- The group identifier for a User's group snapshot
    group_id UUID NOT NULL,

    -- The User who owns this encrypted state snapshot (only for the user to retrieve + update)
    user_id TEXT NOT NULL REFERENCES users(uid) ON DELETE CASCADE,

    -- Monotonically increasing epoch number to determine most updated group state
    epoch BIGINT NOT NULL,

    -- The opaque encrypted blob containing the full group state
    encrypted_content BYTEA NOT NULL,
    encoding TEXT NOT NULL DEFAULT 'base64',

    UNIQUE(user_id, group_id)
);
