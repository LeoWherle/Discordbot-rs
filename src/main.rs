use dotenv::dotenv;
use mcping::get_status;
use serenity::all::ActivityData;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::{GatewayIntents, Ready};
use serenity::prelude::*;
use std::env;
use std::time::Duration;

const UPDATE_TIME_SEC: u64 = 60;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        println!("Bot is connected!");

        // Update the bot's activity with Minecraft server status
        update_status(ctx).await;
    }

    async fn message(&self, ctx: Context, msg: Message) {
        // Respond to the "!status" command
        if msg.content == "!status" {
            let server_status = get_minecraft_server_status().await;

            let content = match server_status {
                Some(status) => status,
                None => "Failed to retrieve server status.".to_string(),
            };

            if let Err(why) = msg.channel_id.say(&ctx.http, content).await {
                println!("Error sending message: {:?}", why);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok(); // Load .env if exists

    // Get the bot token from environment variables
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Add intents
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    // Create the serenity client
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    // Start the client
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

// Function to update bot's activity
async fn update_status(ctx: Context) {
    loop {
        // Query Minecraft server status every 60 seconds
        if let Some(status) = get_minecraft_server_status().await {
            let activity: ActivityData = ActivityData {
                name: status,
                kind: serenity::model::gateway::ActivityType::Playing,
                url: None,
                state: None,
            };
            ctx.set_activity(Some(activity));
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(UPDATE_TIME_SEC)).await;
    }
}

// Function to query the Minecraft server using mcping
use tokio::task;

async fn get_minecraft_server_status() -> Option<String> {
    // Change this to your server's IP address
    let server_address = env::var("SERVER_IP").expect("Expect a server IP");
    let timeout = Duration::from_secs(10);

    // Run the blocking operation in a separate thread
    let result = task::spawn_blocking(move || get_status(&server_address, timeout)).await;

    match result {
        Ok(Ok((latency, status))) => Some(format!(
            "✅ {} players online (latency: {} ms)",
            status.players.online, latency
        )),
        Ok(Err(_)) => Some(format!("❌ Server currently closed")),
        Err(e) => {
            println!("Error in task: {:?}", e);
            None
        }
    }
}
