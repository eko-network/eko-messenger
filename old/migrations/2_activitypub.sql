CREATE TABLE actors (
  id TEXT PRIMARY KEY,
  is_local BOOLEAN NOT NULL,
  inbox_url TEXT NOT NULL,
  outbox_url TEXT NOT NULL
);

CREATE TYPE activity_type AS ENUM(
  'Create',
  'Delivered',
  'Reject',
  'Confirm',
  'Take'
);

CREATE TABLE inbox_activities (
  id TEXT PRIMARY KEY,
  type activity_type NOT NULL,
  activity_json JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  first_delivery_at TIMESTAMPTZ
);

CREATE TABLE message_entries (
  from_did TEXT NOT NULL,
  to_did UUID NOT NULL REFERENCES devices (did) ON DElETE CASCADE,
  activity_id TEXT NOT NULL REFERENCES inbox_activities (id) ON DElETE CASCADE,
  content BYTEA NOT NULL,
  PRIMARY KEY (activity_id, to_did)
);

CREATE TABLE deliveries (
  activity_id TEXT NOT NULL REFERENCES inbox_activities (id) ON DElETE CASCADE,
  to_did UUID NOT NULL REFERENCES devices (did) ON DElETE CASCADE,
  PRIMARY KEY (activity_id, to_did)
);

CREATE OR REPLACE FUNCTION cleanup_activity_on_delivery_delete () RETURNS TRIGGER AS $$
BEGIN
    -- Check if any other deliveries still exist for this specific activity
    IF NOT EXISTS (
        SELECT 1 FROM deliveries 
        WHERE activity_id = OLD.activity_id
    ) THEN
        -- No more deliveries left: Delete the activity.
        -- This will CASCADE and delete related message_entries automatically.
        DELETE FROM inbox_activities 
        WHERE id = OLD.activity_id;
    ELSE
        -- Other deliveries still exist: Just delete the specific message entry
        -- for the device that just had its delivery record removed.
        DELETE FROM message_entries 
        WHERE activity_id = OLD.activity_id 
          AND to_did = OLD.to_did;
    END IF;

    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_cleanup_deliveries
AFTER DELETE ON deliveries FOR EACH ROW
EXECUTE FUNCTION cleanup_activity_on_delivery_delete ();
