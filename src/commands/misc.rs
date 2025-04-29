use poise::CreateReply;
use serenity::all::{CreateMessage, UserId};

use super::{add_role_to_user, has_perms, CmdContext, Error};
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
    let user_id = ctx.author().id;
    let user = ctx
        .data()
        .conn()
        .await
        .get_user_by_id(ctx.author().id)
        .await?;

    let mut roles_given = Vec::new();

    if user.is_verified() {
        add_role_to_user(
            ctx.serenity_context(),
            user_id,
            &config().server.member_role,
        )
        .await?;
        roles_given.push(config().server.member_role.to_string());
    }

    if let Some(rank_name) = user.rank.rank_name() {
        add_role_to_user(ctx.serenity_context(), user_id, rank_name).await?;
        roles_given.push(rank_name.to_string());
    }

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
