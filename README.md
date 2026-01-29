# WIP: Eko Messages
A standalone, end-to-end encrypted (E2EE) messaging application.

This repository contains the backend server for Eko Messenger, a standalone, end-to-end encrypted (E2EE) messaging application that uses the Eko Social app for authentication.

The server is written in Rust and uses the ActivityPub protocol for federation. While this server powers the Eko Messenger application, it also serves as a reference implementation for a secure, E2EE messaging protocol over ActivityPub.

# [Specification](https://github.com/eko-network/eko-messenger/blob/main/SPECIFICATION.md)
The specification is found [here](https://github.com/eko-network/eko-messenger/blob/main/SPECIFICATION.md)

## General Protocol
Examples are found [here](https://github.com/eko-network/eko-messenger/blob/main/EXAMPLES.md)

# Developing
## Requirements
For none nix users:
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

## Testing
By default, tests run with postgres and a test identity provider (so no Firebase is required):
```
cargo test
```

Run Firebase integration tests (requires `FIREBASE_API_KEY` and test user credentials):
```
cargo test --no-default-features --features integration-firebase
```
