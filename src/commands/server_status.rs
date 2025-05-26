use crate::{Context, Error};
use mcping::get_status;
use poise::CreateReply;
use std::time::Duration;
use tokio::task;

#[poise::command(slash_command)]
pub async fn server_status(ctx: Context<'_>) -> Result<(), Error> {
    // Acknowledge the interaction immediately
    ctx.defer_ephemeral().await?;

    let response = get_minecraft_server_status_with_players(ctx, Duration::from_millis(2500)).await;

    match response {
        Some(response) => {
            ctx.send(CreateReply::default().content(response).ephemeral(true))
                .await?;
        }
        None => {
            ctx.send(
                CreateReply::default()
                    .content("❌ Failed to retrieve server status. Is the server IP set? Use /set_server_ip as an admin.")
                    .ephemeral(true),
            )
            .await?;
        }
    }

    Ok(())
}

pub async fn get_minecraft_server_status_with_players(
    ctx: Context<'_>,
    timeout: Duration,
) -> Option<String> {
    get_minecraft_server_status_internal(ctx, timeout, true).await
}

async fn get_minecraft_server_status_internal(
    ctx: Context<'_>,
    timeout: Duration,
    include_players: bool,
) -> Option<String> {
    let guild_id = ctx.guild_id()?;
    let server_ips = ctx.data().server_ips.lock().await;
    let server_address =
        match server_ips.get(&guild_id) {
            Some(addr) => addr.clone(),
            None => return Some(
                "❌ No server IP set for this server. Please ask an admin to use /set_server_ip."
                    .to_owned(),
            ),
        };
    let result = task::spawn_blocking(move || get_status(&server_address, timeout))
        .await
        .ok()?;

    match result {
        Ok((latency, status)) => {
            let players_string = status.players.sample.as_ref().map(|players| {
                players
                    .iter()
                    .map(|p| format!("- {}", p.name))
                    .collect::<Vec<String>>()
                    .join("\n")
            });
            let response = format!(
                "✅ {} players online (latency: {} ms){}",
                status.players.online,
                latency,
                if include_players {
                    players_string
                        .map(|players| format!(":\n{}", players))
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            );

            Some(response)
        }
        Err(_) => Some("❌ Server currently closed".to_owned()),
    }
}
