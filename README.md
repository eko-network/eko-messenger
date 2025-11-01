# eko-messages

A command-line direct messaging client using the Matrix protocol, written in Rust.

## Features

- Send direct messages to Matrix users
- Listen for incoming messages in real-time
- Simple command-line interface
- Secure Matrix protocol communication

## Installation

### Prerequisites

- Rust 1.70 or later
- A Matrix account (you can create one at https://matrix.org)

### Building from source

```bash
git clone https://github.com/ericbreh/eko-messages.git
cd eko-messages
cargo build --release
```

## Configuration

1. Copy the example environment file:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` and add your Matrix credentials:
   ```
   MATRIX_HOMESERVER=https://matrix.org
   MATRIX_USERNAME=your_username
   MATRIX_PASSWORD=your_password
   ```

## Usage

### Send a direct message

```bash
cargo run -- send --to "@recipient:matrix.org" --message "Hello from eko-messages!"
```

Or using the compiled binary:

```bash
./target/release/eko-messages send --to "@recipient:matrix.org" --message "Hello!"
```

### Listen for messages

```bash
cargo run -- listen
```

Or using the compiled binary:

```bash
./target/release/eko-messages listen
```

Press Ctrl+C to stop listening.

## Examples

Send a message:
```bash
cargo run -- send -t "@alice:matrix.org" -m "Hi Alice, how are you?"
```

Listen for incoming messages:
```bash
cargo run -- listen
```

## Security

- Never commit your `.env` file with real credentials
- Use environment variables or secure credential management in production
- The `.env` file is already excluded in `.gitignore`

## License

See LICENSE file for details.
