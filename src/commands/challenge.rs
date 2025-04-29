use poise::CreateReply;
use serenity::all::{CreateEmbed, CreateMessage};
use serenity::builder::CreateForumPost;

use crate::commands::competition::get_competition_from_ctx;
use crate::commands::{CmdContext, Error};
use crate::db::{Challenge, ChallengeType};

/// Creates a new thread for a challenge.
#[poise::command(slash_command)]
pub async fn challenge(
    ctx: CmdContext<'_>,
    #[description = "The name of the challenge"] name: String,
    #[description = "Type category of the challenge"] category: ChallengeType,
) -> Result<(), Error> {
    let competition = get_competition_from_ctx(&ctx).await?;
    let forum = competition
        .channel_id
        .to_channel(ctx)
        .await?
        .guild()
        .expect("You are not in a competition channel.");

    // Automatically add the `unsolved` tag and the tag corresponding to the challenge's category.
    let tag_ids = forum
        .available_tags
        .iter()
        .filter(|t| t.name == category.to_string() || t.name == "unsolved")
        .map(|t| t.id);

    let channel_embed = CreateEmbed::new().color(0xc22026).description(&format!(
        "Discussion for **{category}/{name}**. See **Credentials** for CTF credentials."
    ));

    let thread = forum
        .create_forum_post(
            ctx,
            CreateForumPost::new(
                format!("{category}/{name}"),
                CreateMessage::new().add_embed(channel_embed),
            )
            .set_applied_tags(tag_ids),
        )
        .await?;

    let mut conn = ctx.data().conn().await;

    let challenge = Challenge {
        id: 0,
        competition_id: competition.channel_id,
        name: name.clone(),
        category,
        channel_id: Some(thread.id),
    };
    conn.create_challenge(challenge).await?;

    conn.commit().await?;

    let success_embed = CreateEmbed::new().color(0xc22026).description(&format!(
        "Created channel for **{category}/{name}**.\n→ {thread}"
    ));

    ctx.send(CreateReply {
        embeds: vec![success_embed],
        ..Default::default()
    })
    .await?;

    Ok(())
}
