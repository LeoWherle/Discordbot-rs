use crate::{Context, Error};

#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn set_server_ip(
    ctx: Context<'_>,
    #[description = "The Minecraft server IP address"] ip: String,
) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("This command can only be used in a server.")
                .await?;
            return Ok(());
        }
    };
    {
        let mut data = ctx.data().server_ips.lock().await;
        data.insert(guild_id, ip.clone());
        // drop the lock
    }

    ctx.data().db_handler.set_ip(guild_id, ip.clone()).await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to set server IP: {}", e),
        )
    })?;
    ctx.say(format!("Server IP set to `{}` for this server.", ip))
        .await?;
    Ok(())
}
