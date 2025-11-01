use std::collections::HashSet;

use anyhow::Context;
use serenity::all::{
    Builder, ChannelFlags, ChannelId, ChannelType, CreateButton, CreateChannel, CreateEmbed,
    CreateForumTag, CreateMessage, EditChannel, EditMessage, EditThread, ForumEmoji,
    PermissionOverwrite, Permissions, ReactionType,
};
use serenity::builder::CreateForumPost;

use crate::config::config;
use crate::db::{ActiveCtf, BingoSquare, Challenge, Competition};

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
    let ctf_category_id = config().server.ctf_category_id;
    let channels = ctx
        .guild()
        .ok_or(anyhow::anyhow!("Failed to get guild"))?
        .channels
        .clone();
    let roles = ctx
        .guild()
        .ok_or(anyhow::anyhow!("Failed to get guild"))?
        .roles
        .clone();
    let everyone = roles
        .values()
        .find(|role| role.name == "@everyone")
        .ok_or(anyhow::anyhow!("\\@everyone role not found"))?;
    let officers = roles
        .values()
        .find(|role| role.name == config().server.officer_role)
        .ok_or(anyhow::anyhow!("officer role not found"))?;

    let live_ctfs: HashSet<ChannelId> = channels
        .keys()
        .filter(|id| channels[id].parent_id == Some(ctf_category_id))
        .copied()
        .collect();

    if live_ctfs.iter().any(|ctf| channels[ctf].name == name) {
        return Err(anyhow::anyhow!("CTF channel already exists"));
    }

    // Defer response because channel setup may take longer than 3 seconds
    ctx.defer().await?;

    if !has_perms(&ctx).await {
        return Err(anyhow::anyhow!(
            "You do not have permissions to create a competition."
        ));
    }

    // TODO: prettier error
    // Create forum channel
    let creds_str =
        &format!("**{name}**\n{url}\n\n**Username**: {username}\n**Password**: {password}");
    let mut forum = CreateChannel::new(&name)
        .category(ctf_category_id)
        .position(1)
        .kind(ChannelType::Forum)
        .default_reaction_emoji(ForumEmoji::Id(config().server.ctf_default_emoji_id))
        .topic(creds_str) // Post guidelines for forum channel
        // deny access to everyone except officers by default
        .permissions([
            PermissionOverwrite {
                kind: serenity::all::PermissionOverwriteType::Role(everyone.id),
                allow: Permissions::empty(),
                deny: Permissions::VIEW_CHANNEL,
            },
            PermissionOverwrite {
                kind: serenity::all::PermissionOverwriteType::Role(officers.id),
                allow: Permissions::VIEW_CHANNEL,
                deny: Permissions::empty(),
            },
        ])
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
        .field("Username", username, false)
        .field("Password", password, false);

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

    let join_channel = &channels[&config().server.ctf_join_channel];
    let active = ctx.data().conn().await.get_active_ctfs().await?;
    let send_join_message = async || -> Result<_, anyhow::Error> {
        join_channel
            .send_message(ctx, {
                let mut to_send = CreateMessage::new().content("Join an active ctf:");
                for ctf in &active {
                    to_send = to_send.button(
                        CreateButton::new(format!("{}", &ctf.channel_id))
                            .label(format!("Play in {}", &ctf.name)),
                    );
                }
                to_send
            })
            .await
            .map_err(|e| anyhow::anyhow!(e))
    };

    let mut join_message = match join_channel.last_message_id {
        Some(id) => {
            let msg = join_channel.message(ctx, id).await;
            match msg {
                Ok(msg) => Ok(msg),
                Err(_) => send_join_message().await,
            }
        }
        None => send_join_message().await,
    }?;

    let join_id = join_message.id;
    join_message
        .edit(ctx, {
            let mut to_send = EditMessage::new();
            for ctf in &active {
                to_send = to_send.button(
                    CreateButton::new(ctf.channel_id.to_string())
                        .label(format!("Play in {}", &ctf.name)),
                );
            }
            to_send = to_send.button(
                CreateButton::new(format!("{}", &forum.id)).label(format!("Play in {}", &name)),
            );
            to_send
        })
        .await?;

    let mut conn = ctx.data().conn().await;

    let competition = Competition {
        channel_id: forum.id,
        name: name.clone(),
        bingo: BingoSquare::Free.into(),
    };
    conn.create_competition(competition).await?;

    conn.create_active_ctf(ActiveCtf {
        join_id,
        channel_id: forum.id,
        name: name.clone(),
    })
    .await?;

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
