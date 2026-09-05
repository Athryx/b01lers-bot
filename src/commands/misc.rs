use poise::CreateReply;
use serenity::all::{CreateMessage, UserId};

use super::{has_perms, restore_user_roles, CmdContext, Error};
use crate::config::config;

/// Displays the welcome message
#[poise::command(slash_command)]
pub async fn welcome(ctx: CmdContext<'_>) -> Result<(), Error> {
    ctx.say(&config().server.welcome_message).await?;

    Ok(())
}

/// Gives you your current rank and verified roles in case those were lost
#[poise::command(slash_command)]
pub async fn get_roles(ctx: CmdContext<'_>) -> Result<(), Error> {
    let user = ctx
        .data()
        .conn()
        .await
        .get_user_by_id(ctx.author().id)
        .await?;

    let roles_given = restore_user_roles(ctx.serenity_context(), &user).await?;

    if roles_given.len() > 0 {
        ctx.say(format!("Gave roles `{}`", roles_given.join(", ")))
            .await?;
    } else {
        ctx.say("You don't have any roles to get").await?;
    }

    Ok(())
}

#[poise::command(slash_command)]
pub async fn dm(
    ctx: CmdContext<'_>,
    #[description = "User to send message to"] user: UserId,
    #[description = "Message to send"] message: String,
) -> Result<(), Error> {
    if !has_perms(&ctx).await {
        return Err(anyhow::anyhow!("You do not have permissions to send a dm."));
    }

    let message = CreateMessage::new().content(message);

    user.direct_message(ctx, message).await?;

    let command_reply = CreateReply::default()
        .content("Message sent")
        .ephemeral(true);

    ctx.send(command_reply).await?;

    Ok(())
}
