mod commands;
mod events;

use dotenv::dotenv;
use poise::serenity_prelude as serenity;
use std::{collections::HashMap, env, sync::Arc};

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Arc<Data>, Error>;

// Custom user data passed to all command functions
pub struct Data {
    pub server_ips: tokio::sync::Mutex<HashMap<serenity::all::GuildId, String>>,
}

struct BotData;

impl serenity::prelude::TypeMapKey for BotData {
    type Value = Arc<Data>;
}

async fn on_error(error: poise::FrameworkError<'_, Arc<Data>, Error>) {
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

#[tokio::main]
async fn main() {
    // Initialize logging
    dotenv().ok();
    env_logger::init();

    // Define bot options
    let options = poise::FrameworkOptions {
        commands: vec![commands::server_status(), commands::set_server_ip()],
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
                let data = Arc::new(Data {
                    server_ips: tokio::sync::Mutex::new(HashMap::new()),
                });
                ctx.data.write().await.insert::<BotData>(data.clone());
                Ok(data)
            })
        })
        .options(options)
        .build();

    // Get the bot token from the environment
    let token = env::var("DISCORD_TOKEN")
        .expect("Missing `DISCORD_TOKEN` environment variable. Please set it.");

    // Define the bot's gateway intents
    let intents = serenity::GatewayIntents::non_privileged();

    // Create and start the Discord client
    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        // .event_handler(events::Handler) // Currently disabled as the status is not unique for each guild
        .await
        .expect("Failed to create Discord client");

    client.start().await.expect("Failed to start bot");
}
