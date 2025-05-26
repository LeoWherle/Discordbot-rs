use std::sync::Arc;
use crate::{Context, Error};
use crate::Data;
use ::serenity::all::ActivityData;
use mcping::get_status;
use poise::{serenity_prelude as serenity, CreateReply};
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

pub async fn update_bot_status(ctx: &serenity::Context, data: &Arc<Data>) {
    loop {
        // For each guild, update the bot's activity with its server status
        let server_ips = data.server_ips.lock().await;
        for (_guild_id, server_ip) in server_ips.iter() {
            let status_message = {
                let ip = server_ip.clone();
                let result = task::spawn_blocking(move || get_status(&ip, Duration::from_secs(10))).await.ok();
                match result {
                    Some(Ok((latency, status))) => {
                        let players_string = status.players.sample.as_ref().map(|players| {
                            players
                                .iter()
                                .map(|p| format!("- {}", p.name))
                                .collect::<Vec<String>>()
                                .join("\n")
                        });
                        Some(format!(
                            "✅ {} players online (latency: {} ms){}",
                            status.players.online,
                            latency,
                            players_string.map(|players| format!(":\n{}", players)).unwrap_or_default()
                        ))
                    }
                    _ => Some("❌ Server currently closed".to_string()),
                }
            };
            if let Some(status) = status_message {
                ctx.set_activity(Some(ActivityData {
                    name: status,
                    kind: serenity::model::gateway::ActivityType::Playing,
                    url: None,
                    state: None,
                }));
            }
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

pub async fn get_minecraft_server_status_with_players(ctx: Context<'_>, timeout: Duration) -> Option<String> {
    get_minecraft_server_status_internal(ctx, timeout, true).await
}

async fn get_minecraft_server_status_internal(
    ctx: Context<'_>,
    timeout: Duration,
    include_players: bool,
) -> Option<String> {
    let guild_id = ctx.guild_id()?;
    let server_ips = ctx.data().server_ips.lock().await;
    let server_address = match server_ips.get(&guild_id) {
        Some(addr) => addr.clone(),
        None => return Some("❌ No server IP set for this server. Please ask an admin to use /set_server_ip.".to_string()),
    };
    let result = task::spawn_blocking(move || get_status(&server_address, timeout)).await.ok()?;

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
        Err(_) => Some(format!("❌ Server currently closed")),
    }
}

#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn set_server_ip(
    ctx: Context<'_>,
    #[description = "The Minecraft server IP address"] ip: String,
) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("This command can only be used in a server.").await?;
            return Ok(());
        }
    };
    ctx.data().server_ips.lock().await.insert(guild_id, ip.clone());
    ctx.say(format!("Server IP set to `{}` for this server.", ip)).await?;
    Ok(())
}
