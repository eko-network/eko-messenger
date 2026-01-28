CREATE TABLE users (
  uid TEXT PRIMARY KEY,
  username TEXT UNIQUE NOT NULL,
  email TEXT UNIQUE NOT NULL,
  oidc_issuer TEXT,
  oidc_sub TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_users_oidc_identity 
ON users (oidc_issuer, oidc_sub) 
WHERE oidc_issuer IS NOT NULL AND oidc_sub IS NOT NULL;
