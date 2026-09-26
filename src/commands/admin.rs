use anyhow::Context;
use serenity::all::prelude::Mentionable;
use serenity::all::{ChannelId, CreateEmbed};

use super::has_perms;
use crate::commands::pagination::send_pagination;
use crate::commands::{CmdContext, Error};
use crate::config::config;
use crate::db::User;

/// Ensures the command with the given `ctx` is run by user with admin permissions in the admin channel
async fn check_admin_command_perms(ctx: CmdContext<'_>) -> anyhow::Result<()> {
    match ctx.guild_channel().await {
        Some(channel_id) if channel_id.id == config().server.admin_channel => {
            if has_perms(&ctx).await {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "You do not have permissions to run an admin command."
                ))
            }
        }
        _ => Err(anyhow::anyhow!("You are not inside the admin channel.")),
    }
}

/// Views everyone who participated in a CTF.
#[poise::command(slash_command)]
pub async fn participants(
    ctx: CmdContext<'_>,
    #[description = "Channel of CTF to view members for"] ctf: ChannelId,
) -> Result<(), Error> {
    check_admin_command_perms(ctx).await?;

    let mut conn = ctx.data().conn().await;

    let competition = conn
        .get_competition(ctf)
        .await
        .context("Supplied channel is not a valid CTF channel")?;

    let participants = conn
        .get_ctf_participants(ctf)
        .await
        .context("Unable to fetch participants in CTF channel")?;

    let render = |_page, users_to_render: &[User]| {
        let mut user_list = String::new();

        for user in users_to_render {
            let user_email = match user.email.as_ref() {
                Some(email) => email.as_str(),
                None => "no email available",
            };

            user_list.push_str(&format!("{} ({})\n", user.id.mention(), user_email));
        }

        CreateEmbed::new()
            .title("CTF Participants")
            .description(format!(
                "Participants who joined CTF `{}`",
                competition.name
            ))
            .color(0xc22026)
            .field("Participants", user_list, true)
    };

    send_pagination(ctx, render, &participants, 10).await?;

    Ok(())
}
