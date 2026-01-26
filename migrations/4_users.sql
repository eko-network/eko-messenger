CREATE TABLE users (
  uid TEXT PRIMARY KEY,
  username TEXT UNIQUE NOT NULL,
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  oidc_issuer TEXT,
  oidc_sub TEXT
);

CREATE UNIQUE INDEX idx_users_oidc_identity 
ON users (oidc_issuer, oidc_sub) 
WHERE oidc_issuer IS NOT NULL AND oidc_sub IS NOT NULL;
