mod commands;
mod db;
mod events;

use db::DbRequest;
use dotenv::dotenv;
use poise::serenity_prelude as serenity;
use std::{collections::HashMap, env, sync::Arc};
use tokio::sync::mpsc;
use tracing::error;

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Arc<Data>, Error>;

type IpTable = HashMap<serenity::all::GuildId, String>;

pub struct Data {
    pub server_ips: tokio::sync::Mutex<IpTable>,
    pub db_handler: db::DbHandler,
}

struct BotData;
impl serenity::prelude::TypeMapKey for BotData {
    type Value = Arc<Data>;
}

impl Data {
    pub fn new(db_tx: mpsc::Sender<DbRequest>) -> Self {
        Self {
            server_ips: tokio::sync::Mutex::new(HashMap::new()),
            db_handler: db::DbHandler::new(db_tx),
        }
    }

    pub async fn load_from_db(&self) -> Result<(), String> {
        let result = self.db_handler.reqw_load_from_db().await?;
        let mut map = self.server_ips.lock().await;
        map.clear();
        for (guild_id, ip) in result {
            map.insert(guild_id, ip);
        }
        Ok(())
    }
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
    let db_path = env::var("DISCORD_BOT_DB_FILE").unwrap_or_else(|_| "server_ips.db".to_string());
    let token = env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN env var");
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

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

    // Create the database connection pool
    let mut db_worker = db::DbWorker::new(&db_path).await.unwrap_or_else(|e| {
        error!(?e, "Failed to load server IPs from DB");
        std::process::exit(1);
    });
    db_worker.load_from_db().await.unwrap_or_else(|e| {
        error!(?e, "Failed to initialize DB worker");
        std::process::exit(1);
    });

    // Set up the bot framework
    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                let (db_tx, db_rx) = mpsc::channel(16);

                tokio::spawn(async move {
                    db_worker.run(db_rx).await.unwrap_or_else(|e| {
                        error!(?e, "DB worker encountered an error");
                    });
                });
                let data = Arc::new(Data::new(db_tx));
                if let Err(e) = data.load_from_db().await {
                    error!(?e, "Failed to load server IPs from DB");
                }
                ctx.data.write().await.insert::<BotData>(data.clone());
                Ok(data)
            })
        })
        .options(options)
        .build();

    let intents = serenity::GatewayIntents::non_privileged();
    let mut client = serenity::Client::builder(token, intents)
        .framework(framework)
        // .event_handler(events::Handler) // Currently disabled as the status is not unique for each guild
        .await
        .expect("Error creating client");

    if let Err(err) = client.start_autosharded().await {
        println!("Error starting client: {:?}", err);
    }
}
