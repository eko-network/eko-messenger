CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE devices (
  did UUID PRIMARY KEY,
  uid TEXT NOT NULL,
  is_approved BOOL NOT NULL DEFAULT false,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE device_actions (
  did UUID NOT NULL,
  prev BYTEA,
  uid TEXT NOT NULL,
  is_add BOOL NOT NULL,
  identity_key BYTEA,
  registration_id INTEGER,
  device_name TEXT,
  approved_by_did TEXT,
  approval_signature BYTEA,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (did, is_add),
  CONSTRAINT check_action_type CHECK (
    (
      is_add IS TRUE
      AND identity_key IS NOT NULL
      AND registration_id IS NOT NULL
    )
    OR (
      is_add IS FALSE
      AND identity_key IS NULL
      AND registration_id IS NULL
    )
  ),
  CONSTRAINT check_approval CHECK (
    -- Genesis node: no approval needed
    (
      prev IS NULL
      AND approved_by_did IS NULL
      AND approval_signature IS NULL
    )
    OR
    -- Approved device: both fields required
    (
      prev IS NOT NULL
      AND approved_by_did IS NOT NULL
      AND approval_signature IS NOT NULL
    )
  )
);

CREATE OR REPLACE FUNCTION block_update_func () RETURNS TRIGGER AS $$
BEGIN
RAISE EXCEPTION 'Updates are not allowed on the table: %',
TG_TABLE_NAME;
RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_block_update_device_actions BEFORE
UPDATE ON device_actions FOR EACH ROW
EXECUTE FUNCTION block_update_func ();

CREATE OR REPLACE FUNCTION update_devices_from_actions () RETURNS TRIGGER AS $$
DECLARE
device_count INT;
new_is_approved BOOl;
BEGIN
IF NEW.is_add THEN
-- Determine status: auto-approve if genesis or approved, else pending
SELECT
    COUNT(*) INTO device_count
FROM
    device_actions
WHERE
    uid = NEW.uid
    AND is_add = TRUE;

IF device_count = 1 THEN
-- First device (genesis): auto-approve
new_is_approved := TRUE;

ELSIF NEW.approved_by_did IS NOT NULL THEN
-- Approved by another device
new_is_approved := TRUE;

ELSE
-- Waiting for approval
new_is_approved := TRUE; --FIXME this should be FALSE after approval proccess is created
END IF;

-- Update approval status of existing device
-- Device should already exist from login insert
UPDATE
    devices
SET
    is_approved = new_is_approved
WHERE
    did = NEW.did;

ELSE
-- Revoke device: delete from devices table
DELETE FROM
    devices
WHERE
    did = NEW.did;

END IF;

RETURN NEW;

END;

$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_update_devices
AFTER INSERT ON device_actions FOR EACH ROW
EXECUTE FUNCTION update_devices_from_actions ();

CREATE TABLE refresh_tokens (
  token UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  did UUID NOT NULL UNIQUE REFERENCES devices (did) ON DELETE CASCADE,
  ip_address TEXT NOT NULL,
  user_agent TEXT NOT NULL,
  issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE pre_keys (
  id SERIAL PRIMARY KEY,
  did UUID NOT NULL REFERENCES devices (did) ON DELETE CASCADE,
  key_id INTEGER NOT NULL,
  KEY BYTEA NOT NULL,
  UNIQUE (did, key_id)
);

CREATE TABLE signed_pre_keys (
  did UUID NOT NULL REFERENCES devices (did) ON DELETE CASCADE,
  key_id INTEGER NOT NULL,
  KEY BYTEA NOT NULL,
  signature BYTEA NOT NULL,
  PRIMARY KEY (did, key_id)
);

CREATE INDEX idx_devices_user_id ON devices (uid);

CREATE INDEX idx_devices_approved ON devices (is_approved);

CREATE INDEX idx_device_actions_uid_created_at ON device_actions (uid, created_at DESC);

CREATE INDEX idx_device_actions_pending_approval ON device_actions (approved_by_did)
WHERE
  approved_by_did IS NULL;

CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens (expires_at);

-- Ensure only ONE genesis node per user (prevents race condition)
-- CREATE UNIQUE INDEX idx_genesis_nodes ON device_actions (uid)
-- WHERE
--   prev IS NULL;
