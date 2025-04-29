use serenity::all::EditChannel;

use crate::commands::competition::get_competition_from_ctx;
use crate::commands::{has_perms, CmdContext, Error};
use crate::config::config;

/// Archives the current competition channel.
#[poise::command(slash_command)]
pub async fn archive(ctx: CmdContext<'_>) -> Result<(), Error> {
    // category where archived ctf channels are sent
    let archived_category_id = config().server.archived_ctf_category_id;

    if !has_perms(&ctx).await {
        return Err(anyhow::anyhow!(
            "You do not have permissions to archive a competition."
        ));
    }

    // Ensure command is being run within a competition channel, and the competition is not already archived.
    let competition = get_competition_from_ctx(&ctx).await?;

    let mut channel = competition
        .channel_id
        .to_channel(ctx)
        .await?
        .guild()
        .expect("You are not inside a competition channel.");

    if channel
        .parent_id
        .is_some_and(|id| id == archived_category_id)
    {
        return Err(anyhow::anyhow!("This competition is already archived!"));
    }

    // Move the channel to the archived category.
    channel
        .edit(ctx, EditChannel::new().category(archived_category_id))
        .await?;

    ctx.say(format!("Archived **{}**.", competition.name))
        .await?;

    Ok(())
}
