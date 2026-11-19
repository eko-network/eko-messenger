-- One token per device for its notification service, which reads the inbox
-- and acknowledges messages while the app is not running. Only a hash is
-- kept. A token is good for as long as its device is on the user's account.
CREATE TABLE notification_tokens (
  did UUID PRIMARY KEY,
  uid UUID NOT NULL,
  token_hash BYTEA NOT NULL UNIQUE,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
