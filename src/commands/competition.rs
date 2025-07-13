use anyhow::Context;
use serenity::all::{
    Builder, ChannelFlags, ChannelId, ChannelType, CreateChannel, CreateEmbed, CreateForumTag,
    CreateMessage, EditChannel, EditThread, ForumEmoji, ReactionType,
};
use serenity::builder::CreateForumPost;

use crate::config::config;
use crate::db::{BingoSquare, Challenge, Competition};

use super::{has_perms, CmdContext, Error};

/// Creates a new ctf competition channel.
#[poise::command(slash_command)]
pub async fn competition(
    ctx: CmdContext<'_>,
    #[description = "Name of the ctf"] name: String,
    #[description = "Url of ctf website"] url: String,
    //#[description = "Description of the ctf"] description: Option<String>,
    #[description = "Team username"] username: String,
    #[description = "Team password or login url"] password: String,
) -> Result<(), Error> {
    // TODO: figure out how to get all channels in a category, so we can check duplicate names

    // Defer response because channel setup may take longer than 3 seconds
    ctx.defer().await?;

    if !has_perms(&ctx).await {
        return Err(anyhow::anyhow!(
            "You do not have permissions to create a competition."
        ));
    }

    if username.contains("`") {
        return Err(anyhow::anyhow!("Username cannot include a backtick (\\`)"));
    }

    if password.contains("`") {
        return Err(anyhow::anyhow!("Password cannot include a backtick (\\`)"));
    }
    let username_esc = format!("`{username}`");
    let password_esc = format!("`{password}`");
    // TODO: prettier error
    // Create forum channel
    let creds_str =
        &format!("**{name}**\n{url}\n\n**Username**: {username_esc}\n**Password**: {password_esc}");
    let mut forum = CreateChannel::new(&name)
        .category(config().server.ctf_category_id)
        .position(0)
        .kind(ChannelType::Forum)
        .default_reaction_emoji(ForumEmoji::Id(config().server.ctf_default_emoji_id))
        .topic(creds_str) // Post guidelines for forum channel
        .execute(ctx, config().server.guild_id)
        .await?;

    // Add category and solved tags to forum channel
    let tags = vec![
        CreateForumTag::new("welcome").emoji(ReactionType::Unicode("🎉".to_string())),
        CreateForumTag::new("web").emoji(ReactionType::Unicode("🌐".to_string())),
        CreateForumTag::new("crypto").emoji(ReactionType::Unicode("🧮".to_string())),
        CreateForumTag::new("pwn").emoji(ReactionType::Unicode("💥".to_string())),
        CreateForumTag::new("rev").emoji(ReactionType::Unicode("🛠️".to_string())),
        CreateForumTag::new("misc").emoji(ReactionType::Unicode("⚙️".to_string())),
        CreateForumTag::new("forensics").emoji(ReactionType::Unicode("🔍".to_string())),
        CreateForumTag::new("osint").emoji(ReactionType::Unicode("🕵️".to_string())),
        CreateForumTag::new("blockchain").emoji(ReactionType::Unicode("⛓".to_string())),
        CreateForumTag::new("programming").emoji(ReactionType::Unicode("👨‍💻".to_string())),
        CreateForumTag::new("jail").emoji(ReactionType::Unicode("🚔".to_string())),
        CreateForumTag::new("unsolved").emoji(ReactionType::Unicode("❌".to_string())),
        CreateForumTag::new("solved").emoji(ReactionType::Unicode("✅".to_string())),
    ];
    forum
        .edit(ctx, EditChannel::new().available_tags(tags))
        .await?;

    // Create post with credentials
    let credentials_embed = CreateEmbed::new()
        .color(0xc22026)
        .title(&format!("{name} credentials"))
        .description(url)
        .field("Username", username_esc, false)
        .field("Password", password_esc, false);

    let mut creds_channel = forum
        .create_forum_post(
            ctx,
            CreateForumPost::new(
                "Credentials + general discussion",
                CreateMessage::new().add_embed(credentials_embed),
            ),
        )
        .await?;

    // Pin credentials / general discussion post
    creds_channel
        .edit_thread(ctx, EditThread::new().flags(ChannelFlags::PINNED))
        .await?;

    // Pin credentials message in creds channel
    if let Some(creds_message_id) = creds_channel.last_message_id {
        let creds_message = creds_channel.message(ctx, creds_message_id).await?;
        creds_message.pin(ctx).await?;
    }

    let mut conn = ctx.data().conn().await;

    let competition = Competition {
        channel_id: forum.id,
        name: name.clone(),
        bingo: BingoSquare::Free.into(),
    };
    conn.create_competition(competition).await?;

    conn.commit().await?;

    ctx.say(format!("Created channel for **{name}**: {forum}"))
        .await?;

    Ok(())
}

pub async fn get_competition_id_from_ctx(ctx: &CmdContext<'_>) -> Result<ChannelId, Error> {
    let Some(thread_channel) = ctx.guild_channel().await else {
        Err(anyhow::anyhow!("You are not inside a competition channel."))?
    };

    // For a forum channel, the competition channel will be the command channel's parent.
    let Some(channel_id) = thread_channel.parent_id else {
        Err(anyhow::anyhow!("You are not inside a competition channel."))?
    };

    Ok(channel_id)
}

/// Gets the competition in the channel the command was invoked from.
pub async fn get_competition_from_ctx(ctx: &CmdContext<'_>) -> Result<Competition, Error> {
    let channel_id = get_competition_id_from_ctx(ctx).await?;

    let competition = ctx
        .data()
        .conn()
        .await
        .get_competition(channel_id)
        .await
        .with_context(|| "You are not inside a competition channel.")?;

    Ok(competition)
}

pub async fn get_challenge_from_ctx(ctx: &CmdContext<'_>) -> Result<Challenge, Error> {
    let Some(thread_channel) = ctx.guild_channel().await else {
        Err(anyhow::anyhow!("You are not inside a challenge channel."))?
    };

    let challenge = ctx
        .data()
        .conn()
        .await
        .get_challenge_by_channel_id(thread_channel.id)
        .await
        .with_context(|| "You are not inside a challenge channel.")?;

    Ok(challenge)
}
