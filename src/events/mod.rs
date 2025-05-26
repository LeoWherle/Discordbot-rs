use crate::{BotData, Data};
use ::serenity::all::ActivityData;
use mcping::get_status;
use poise::serenity_prelude as serenity;
use std::sync::Arc;
use std::time::Duration;
use tokio::task;

pub struct Handler;

#[serenity::async_trait]
impl serenity::EventHandler for Handler {
    async fn ready(&self, ctx: serenity::Context, _data_about_bot: serenity::Ready) {
        let data_lock = ctx.data.read().await;
        if let Some(data) = data_lock.get::<BotData>() {
            let data_clone = Arc::clone(data);
            // Use a reference to the context for the background task
            let ctx_ref = &ctx;
            tokio::spawn(background_status_update(ctx_ref.clone(), data_clone));
        }
    }
}

/// Task that runs in the background to update the bot's status every 30 seconds
pub async fn background_status_update(ctx: serenity::Context, data: Arc<Data>) {
    loop {
        update_bot_status(&ctx, &data).await;
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    }
}

pub async fn update_bot_status(ctx: &serenity::Context, data: &Arc<Data>) {
    // For each guild, update the bot's activity with its server status
    let server_ips = data.server_ips.lock().await;
    // Pick the first server IP (if any) to display as the bot's activity
    if let Some((_guild_id, server_ip)) = server_ips.iter().next() {
        let status_message = {
            let ip = server_ip.clone();
            let result = task::spawn_blocking(move || get_status(&ip, Duration::from_secs(10)))
                .await
                .ok();
            match result {
                Some(Ok((latency, status))) => Some(format!(
                    "✅ {} players online (latency: {} ms)",
                    status.players.online, latency,
                )),
                _ => Some("❌ Server currently closed".to_owned()),
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
    } else {
        ctx.set_activity(Some(ActivityData {
            name: "No server IP set".to_string(),
            kind: serenity::model::gateway::ActivityType::Playing,
            url: None,
            state: None,
        }));
    }
}
