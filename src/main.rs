use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use matrix_sdk::{
    config::SyncSettings,
    ruma::{
        events::room::message::{
            MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent,
        },
        UserId,
    },
    Client, Room, RoomState,
};
use std::env;

#[derive(Parser)]
#[command(name = "eko-messages")]
#[command(about = "A Matrix protocol DM client", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a direct message to a user
    Send {
        /// The Matrix user ID to send to (e.g., @user:matrix.org)
        #[arg(short, long)]
        to: String,

        /// The message content
        #[arg(short, long)]
        message: String,
    },
    /// Listen for incoming messages
    Listen,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();

    // Get configuration from environment variables
    let homeserver =
        env::var("MATRIX_HOMESERVER").context("MATRIX_HOMESERVER environment variable not set")?;
    let username =
        env::var("MATRIX_USERNAME").context("MATRIX_USERNAME environment variable not set")?;
    let password =
        env::var("MATRIX_PASSWORD").context("MATRIX_PASSWORD environment variable not set")?;

    // Create and login to the Matrix client
    let client = login(&homeserver, &username, &password).await?;

    match cli.command {
        Commands::Send { to, message } => {
            send_dm(&client, &to, &message).await?;
        }
        Commands::Listen => {
            listen_for_messages(&client).await?;
        }
    }

    Ok(())
}

/// Login to the Matrix homeserver
async fn login(homeserver: &str, username: &str, password: &str) -> Result<Client> {
    println!("Logging in to {}...", homeserver);

    let client = Client::builder().homeserver_url(homeserver).build().await?;

    client
        .matrix_auth()
        .login_username(username, password)
        .initial_device_display_name("eko-messages")
        .send()
        .await?;

    println!("Logged in as {}", username);

    Ok(client)
}

/// Send a direct message to a user
async fn send_dm(client: &Client, to: &str, message: &str) -> Result<()> {
    let user_id = UserId::parse(to)
        .context("Invalid user ID format. Expected format: @user:homeserver.org")?;

    println!("Creating DM room with {}...", to);

    // Create or get existing DM room
    let room = client
        .create_dm(&user_id)
        .await
        .context("Failed to create DM room")?;

    println!("Sending message...");

    // Send the message
    let content = RoomMessageEventContent::text_plain(message);
    room.send(content).await?;

    println!("Message sent successfully!");

    Ok(())
}

/// Listen for incoming messages
async fn listen_for_messages(client: &Client) -> Result<()> {
    println!("Listening for messages... (Press Ctrl+C to stop)");

    // Set up the message handler
    client.add_event_handler(on_room_message);

    // Start syncing
    let sync_settings = SyncSettings::default();
    client.sync(sync_settings).await?;

    Ok(())
}

/// Handler for incoming room messages
async fn on_room_message(event: OriginalSyncRoomMessageEvent, room: Room) {
    // Only handle messages in joined rooms
    if room.state() != RoomState::Joined {
        return;
    }

    // Extract message text
    let MessageType::Text(text_content) = event.content.msgtype else {
        return;
    };

    let sender = event.sender;
    let message_body = text_content.body;

    // Get room name or fall back to room ID
    let room_name = room
        .display_name()
        .await
        .ok()
        .map(|name| name.to_string())
        .unwrap_or_else(|| room.room_id().to_string());

    println!("[{}] {}: {}", room_name, sender, message_body);
}
