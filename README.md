# WIP: Eko Messages

A standalone, end-to-end encrypted (E2EE) messaging application.

This repository contains the backend server for Eko Messenger, a standalone, end-to-end encrypted (E2EE) messaging application that uses the Eko Social app for authentication.

The server is written in Rust and uses the ActivityPub protocol for federation. While this server powers the Eko Messenger application, it also serves as a reference implementation for a secure, E2EE messaging protocol over ActivityPub.

# Specification

The specification is found [here](https://github.com/eko-network/eko-messenger/blob/main/SPECIFICATION.md)

## General Protocol

Examples are found [here](https://github.com/eko-network/eko-messenger/blob/main/EXAMPLES.md)

# Developing

## Requirements

For non nix users:

* [rust](https://rust-lang.org/tools/install/)
* [postgres](https://www.postgresql.org/download/)
* [process-compose](https://github.com/F1bonacc1/process-compose)
* sqlx: cargo install sqlx-cli

## Starting DB

Note: Must run with the env variables

```
process-compose
```

## Running Rust server

Note: Must run with the env variables

```
cargo run
```

## Self hosted OIDC Configuration (Zitadel)

To test with a self-hosted Zitadel instance:

1. Run `docker compose -f docker-compose.zitadel.yaml up -d`
2. Configure Zitadel at `http://localhost:8081/ui/console?login_hint=zitadel-admin@zitadel.localhost` with password `Password1!`:
   * Create a Project and an Application (Type: WEB, Auth Method: CODE).
   * Set Redirect URI to `http://localhost:3000/auth/v1/oidc/callback`.
   * Enable "Include user's profile info in the ID Token" in Token Settings.
3. Update your `.env` with the credentials. Set `OIDC_ADDITIONAL_AUDIENCES` to your Project ID.

## Testing

By default, tests run with postgres and a test identity provider (so no Firebase is required):

```
cargo test
```

To run the test suite against Postgres instead, set `TEST_STORAGE_BACKEND` or `STORAGE_BACKEND` to `postgres`

Run Firebase integration tests (requires `FIREBASE_API_KEY` and test user credentials):

```
cargo test --features integration-firebase
```
