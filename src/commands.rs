use crate::{Context, Error};
use mcping::get_status;
use std::{env, time::Duration};
use tokio::task;

#[poise::command(slash_command)]
pub async fn server_status(ctx: Context<'_>) -> Result<(), Error> {
    // Acknowledge the interaction immediately
    ctx.defer().await?;

    match get_minecraft_server_status(Duration::from_millis(2500)).await {
        Some(response) => {
            ctx.say(response).await?;
        }
        None => {
            ctx.say("❌ Failed to retrieve server status").await?;
        }
    }

    Ok(())
}

// Non-blocking function to query Minecraft server status
pub async fn get_minecraft_server_status(timeout: Duration) -> Option<String> {
    // Minecraft server address
    let server_address = env::var("SERVER_IP").expect("missing server ip");

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
