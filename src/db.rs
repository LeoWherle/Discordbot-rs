use sqlx::{Pool, Row, Sqlite, SqlitePool};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tracing::error;

use crate::IpTable;

// Custom user data passed to all command functions
#[derive(Debug)]
pub enum DbRequest {
    SetIp {
        guild_id: serenity::all::GuildId,
        ip: String,
        resp: oneshot::Sender<Result<(), String>>,
    },
    GetAllIps {
        resp: oneshot::Sender<HashMap<serenity::all::GuildId, String>>,
    },
    LoadFromDb {
        resp: oneshot::Sender<Result<HashMap<serenity::all::GuildId, String>, String>>,
    },
}

pub struct DbHandler {
    pub db_tx: mpsc::Sender<DbRequest>,
}

impl DbHandler {
    pub fn new(db_tx: mpsc::Sender<DbRequest>) -> Self {
        Self { db_tx }
    }

    pub async fn set_ip(&self, guild_id: serenity::all::GuildId, ip: String) -> Result<(), String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.db_tx
            .send(DbRequest::SetIp {
                guild_id,
                ip,
                resp: resp_tx,
            })
            .await
            .map_err(|e| format!("Send error: {e}"))?;
        resp_rx.await.map_err(|e| format!("Recv error: {e}"))??;
        Ok(())
    }

    pub async fn reqw_load_from_db(
        &self,
    ) -> Result<HashMap<serenity::all::GuildId, String>, String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.db_tx
            .send(DbRequest::LoadFromDb { resp: resp_tx })
            .await
            .map_err(|e| format!("Send error: {e}"))?;
        resp_rx.await.map_err(|e| format!("Recv error: {e}"))?
    }
}

pub struct DbWorker {
    pub pool: Pool<Sqlite>,
    pub cache: IpTable,
}

impl DbWorker {
    pub async fn new(db_path: &str) -> Result<Self, String> {
        let pool = SqlitePool::connect(&format!("sqlite://{}?mode=rwc", db_path))
            .await
            .map_err(|e| format!("Failed to connect to SQLite DB: {e}"))?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS server_ips (guild_id INTEGER PRIMARY KEY, ip TEXT NOT NULL)",
        )
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to create table: {e}"))?;
        Ok(Self {
            pool,
            cache: IpTable::new(),
        })
    }

    pub async fn load_from_db(&mut self) -> Result<(), String> {
        match sqlx::query("SELECT guild_id, ip FROM server_ips")
            .fetch_all(&self.pool)
            .await
        {
            Ok(rows) => {
                for row in rows {
                    let guild_id: i64 = row.get("guild_id");
                    let ip: String = row.get("ip");
                    self.cache
                        .insert(serenity::all::GuildId::from(guild_id as u64), ip);
                }
            }
            Err(e) => error!(?e, "Failed to load initial cache from DB"),
        }
        Ok(())
    }

    pub async fn run(mut self, mut rx: mpsc::Receiver<DbRequest>) -> Result<(), String> {
        while let Some(req) = rx.recv().await {
            match req {
                DbRequest::SetIp { guild_id, ip, resp } => {
                    let res = sqlx::query("INSERT INTO server_ips (guild_id, ip) VALUES (?, ?) ON CONFLICT(guild_id) DO UPDATE SET ip=excluded.ip")
                    .bind(u64::from(guild_id) as i64)
                    .bind(&ip)
                    .execute(&self.pool)
                    .await;
                    match res {
                        Ok(_) => {
                            self.cache.insert(guild_id, ip);
                            let _ = resp.send(Ok(()));
                        }
                        Err(e) => {
                            error!(
                                ?e,
                                "Failed to set server_ip for guild_id: {}",
                                u64::from(guild_id)
                            );
                            let _ = resp.send(Err(format!("DB error: {e}")));
                        }
                    }
                }
                DbRequest::GetAllIps { resp } => {
                    let _ = resp.send(self.cache.clone());
                }
                DbRequest::LoadFromDb { resp } => {
                    let _ = resp.send(Ok(self.cache.clone()));
                }
            }
        }
        Ok(())
    }
}
