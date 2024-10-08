use crate::{Context, Error};
use ::serenity::all::ActivityData;
use mcping::get_status;
use poise::serenity_prelude as serenity;
use std::{env, time::Duration};
use tokio::task;

#[poise::command(slash_command)]
pub async fn server_status(ctx: Context<'_>) -> Result<(), Error> {
    // Acknowledge the interaction immediately
    ctx.defer().await?;

    match get_minecraft_server_status_with_players(Duration::from_millis(2500)).await {
        Some(response) => {
            ctx.say(response).await?;
        }
        None => {
            ctx.say("❌ Failed to retrieve server status").await?;
        }
    }

    Ok(())
}

pub async fn update_bot_status(ctx: &serenity::Context) {
    loop {
        let status_message = get_minecraft_server_status(Duration::from_secs(10)).await;

        let activity = if let Some(players_status) = status_message {
            let activity_data = ActivityData {
                name: players_status,
                kind: serenity::model::gateway::ActivityType::Playing,
                url: None,
                state: None,
            };
            Some(activity_data)
        } else {
            None
        };
        ctx.set_activity(activity);
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

pub async fn get_minecraft_server_status_with_players(timeout: Duration) -> Option<String> {
    get_minecraft_server_status_internal(timeout, true).await
}

pub async fn get_minecraft_server_status(timeout: Duration) -> Option<String> {
    get_minecraft_server_status_internal(timeout, false).await
}

async fn get_minecraft_server_status_internal(
    timeout: Duration,
    include_players: bool,
) -> Option<String> {
    let server_address = env::var("SERVER_IP").expect("missing server ip");
    let result = task::spawn_blocking(move || get_status(&server_address, timeout)).await;

    match result {
        Ok(Ok((latency, status))) => {
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
        Ok(Err(_)) => Some(format!("❌ Server currently closed")),
        Err(e) => {
            println!("Error in task: {:?}", e);
            None
        }
    }
}
