CREATE TABLE messages (
  id SERIAL PRIMARY KEY,
  object_id TEXT,
  delivered BOOL DEFAULT FALSE,
  activity_type TEXT NOT NULL,
  activity JSONB NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE entries (
  id SERIAL PRIMARY KEY,
  message_id INTEGER REFERENCES messages (id) ON DELETE CASCADE,
  did UUID NOT NULL
);

CREATE OR REPLACE FUNCTION public.insert_delivered(
    p_sender_did UUID,
    p_target_object_id TEXT,
    p_delivered_activity JSONB,
    p_delivered_activity_id TEXT,
    p_notify_dids UUID[]
) RETURNS VOID AS $$
DECLARE
    v_orig_message_id INTEGER;
    v_delivered_status BOOL;
    v_new_message_id INTEGER;
BEGIN
    -- Find original message
    SELECT id, delivered INTO v_orig_message_id, v_delivered_status
    FROM messages
    WHERE object_id = p_target_object_id
    LIMIT 1;

    IF v_orig_message_id IS NOT NULL THEN
        -- If not already delivered, insert new activity and notify others
        IF NOT v_delivered_status THEN
            INSERT INTO messages (activity_type, activity, object_id)
            VALUES ('Delivered', p_delivered_activity, p_delivered_activity_id)
            RETURNING id INTO v_new_message_id;

            -- Add entries for notify_dids
            INSERT INTO entries (message_id, did)
            SELECT v_new_message_id, unnest(p_notify_dids);

            -- Mark original as delivered
            UPDATE messages SET delivered = TRUE WHERE id = v_orig_message_id;
        END IF;

        -- Always delete entry for sender device from the original message's inbox
        DELETE FROM entries WHERE message_id = v_orig_message_id AND did = p_sender_did;
    END IF;
END;
$$ LANGUAGE plpgsql;
