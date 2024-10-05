#![warn(clippy::str_to_string)]

mod commands;

use ::serenity::all::ActivityData;
use dotenv::dotenv;
use poise::serenity_prelude as serenity;
use std::{collections::HashMap, env, sync::Mutex, time::Duration};

use commands::get_minecraft_server_status;

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
pub struct Data {
    votes: Mutex<HashMap<String, u32>>,
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e);
            }
        }
    }
}

async fn update_bot_status(ctx: &serenity::Context) {
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

#[tokio::main]
async fn main() {
    // Initialize logging
    dotenv().ok();
    env_logger::init();

    // Define bot options
    let options = poise::FrameworkOptions {
        commands: vec![commands::server_status()],
        on_error: |error| Box::pin(on_error(error)),
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        post_command: |ctx| {
            Box::pin(async move {
                println!("Executed command {}!", ctx.command().qualified_name);
            })
        },
        ..Default::default()
    };

    // Set up the bot framework
    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                let ctx_clone = ctx.clone();
                tokio::spawn(async move {
                    update_bot_status(&ctx_clone).await;
                });
                Ok(Data {
                    votes: Mutex::new(HashMap::new()),
                })
            })
        })
        .options(options)
        .build();

    // Get the bot token from the environment
    let token = env::var("DISCORD_TOKEN")
        .expect("Missing `DISCORD_TOKEN` environment variable. Please set it.");
    let _server_ip = env::var("SERVER_IP")
        .expect("Missing `SERVER_IP` environment variable. Please set it.    ");

    // Define the bot's gateway intents
    let intents = serenity::GatewayIntents::non_privileged();

    // Create and start the Discord client
    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .expect("Failed to create Discord client");

    client.start().await.expect("Failed to start bot");
}
