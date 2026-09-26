use std::collections::HashSet;

use anyhow::Context;
use serenity::all::{
    Builder, ChannelFlags, ChannelId, ChannelType, CreateActionRow, CreateButton, CreateChannel,
    CreateEmbed, CreateForumTag, CreateMessage, EditChannel, EditMessage, EditThread, ForumEmoji,
    MessageId, PermissionOverwrite, PermissionOverwriteType, Permissions, ReactionType,
};
use serenity::builder::CreateForumPost;

use crate::config::config;
use crate::db::{BingoSquare, Challenge, Competition};

use super::{has_perms, CmdContext, Error};

#[poise::command(slash_command, subcommands("new", "edit", "archive"))]
pub async fn competition(_ctx: CmdContext<'_>) -> anyhow::Result<()> {
    Ok(())
}

/// Creates a new ctf competition channel.
#[poise::command(slash_command)]
pub async fn new(
    ctx: CmdContext<'_>,
    #[description = "Name of the ctf"] name: String,
    #[description = "Ctf website URL"] url: String,
    #[description = "Is AI allowed?"] ai_allowed: bool,
    #[description = "Team username"] username: Option<String>,
    #[description = "Team email"] email: Option<String>,
    #[description = "Team password"] password: Option<String>,
    #[description = "Team token or login URL"] token: Option<String>,
) -> Result<(), Error> {
    let ctf_category_id = config().server.ctf_category_id;
    let channels = ctx
        .guild()
        .ok_or(anyhow::anyhow!("Failed to get guild"))?
        .channels
        .clone();
    let roles = ctx
        .guild()
        .ok_or(anyhow::anyhow!("Failed to get roles"))?
        .roles
        .clone();
    let everyone = roles
        .values()
        .find(|role| role.name == "@everyone")
        .ok_or(anyhow::anyhow!("\\@everyone role not found"))?;

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

    // default ids until we make them later, need competition for rendering
    let mut competition = Competition {
        channel_id: ChannelId::default(),
        name: name.clone(),
        bingo: BingoSquare::Free.into(),
        active: true,
        ai_allowed: ai_allowed,
        url: url,
        creds_channel_id: ChannelId::default(),
        creds_message_id: MessageId::default(),
        username: username,
        email: email,
        token: token,
        password: password,
    };

    let forum_topic = render_forum_topic(&competition)?;
    let credentials_embed = render_credentials_embed(&competition)?;

    // TODO: prettier error
    // Create forum channel
    let mut forum = CreateChannel::new(&name)
        .category(ctf_category_id)
        .position(1)
        .kind(ChannelType::Forum)
        .default_reaction_emoji(ForumEmoji::Id(config().server.ctf_default_emoji_id))
        .topic(forum_topic) // Post guidelines for forum channel
        // deny access to everyone
        .permissions([PermissionOverwrite {
            kind: PermissionOverwriteType::Role(everyone.id),
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
        }])
        .execute(ctx, config().server.guild_id)
        .await?;
    competition.channel_id = forum.id;

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
    let mut creds_channel = forum
        .create_forum_post(
            ctx,
            CreateForumPost::new(
                "Credentials + general discussion",
                CreateMessage::new().add_embed(credentials_embed),
            ),
        )
        .await?;
    competition.creds_channel_id = creds_channel.id;

    // Pin credentials / general discussion post
    creds_channel
        .edit_thread(ctx, EditThread::new().flags(ChannelFlags::PINNED))
        .await?;

    // Pin credentials message in creds channel
    if let Some(creds_message_id) = creds_channel.last_message_id {
        competition.creds_message_id = creds_message_id;

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
    conn.create_competition(competition).await?;
    conn.commit().await?;

    ctx.say(format!("Created channel for **{name}**: {forum}"))
        .await?;

    Ok(())
}

/// Edits credential details for the current ctf.
#[poise::command(slash_command)]
pub async fn edit(
    ctx: CmdContext<'_>,
    #[description = "Team username"] username: Option<String>,
    #[description = "Team email"] email: Option<String>,
    #[description = "Team password"] password: Option<String>,
    #[description = "Team token or login URL"] token: Option<String>,
) -> Result<(), Error> {
    if !has_perms(&ctx).await {
        return Err(anyhow::anyhow!(
            "You do not have permissions to edit a competition."
        ));
    }

    // Ensure command is being run within a competition channel, and the competition is not already archived.
    let channel_id = get_competition_id_from_ctx(&ctx).await?;

    let mut conn = ctx.data().conn().await;

    let mut competition = conn.get_competition(channel_id).await?;

    if let Some(value) = username {
        competition.username = Some(value);
    }
    if let Some(value) = email {
        competition.email = Some(value);
    }
    if let Some(value) = password {
        competition.password = Some(value);
    }
    if let Some(value) = token {
        competition.token = Some(value);
    }

    let forum_topic = render_forum_topic(&competition)?;
    let credentials_embed = render_credentials_embed(&competition)?;

    let mut forum = competition
        .channel_id
        .to_channel(ctx)
        .await?
        .guild()
        .ok_or_else(|| anyhow::anyhow!("Competition is not a channel"))?;

    let creds_channel = competition
        .creds_channel_id
        .to_channel(ctx)
        .await?
        .guild()
        .ok_or_else(|| anyhow::anyhow!("Credentials post is not a guild channel"))?;

    let mut creds_message = creds_channel
        .message(ctx, competition.creds_message_id)
        .await?;

    forum
        .edit(ctx, EditChannel::new().topic(forum_topic))
        .await?;

    creds_message
        .edit(ctx, EditMessage::new().embed(credentials_embed))
        .await?;

    conn.update_competition(competition).await?;
    conn.commit().await?;

    ctx.say(format!("Updated competition info")).await?;

    Ok(())
}

/// Archives the current competition channel.
#[poise::command(slash_command)]
pub async fn archive(ctx: CmdContext<'_>) -> Result<(), Error> {
    // category where archived ctf channels are sent
    let archived_category_id = config().server.archived_ctf_category_id;
    let join_channel_id = config().server.ctf_join_channel;
    let join_channel = join_channel_id.to_channel(ctx).await?;

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

    // Remove the channel from active CTFS
    let mut conn = ctx.data().conn().await;
    conn.remove_active_ctf(competition.channel_id).await?;
    conn.commit().await?;

    // Edit button to no longer provide access
    let active = ctx.data().conn().await.get_active_ctfs().await?;
    if let Some(join_message_id) = join_channel.guild().and_then(|guild| guild.last_message_id) {
        let mut join_message = join_channel_id.message(ctx, join_message_id).await?;
        let buttons: Vec<CreateButton> = active
            .iter()
            .filter(|ctf| ctf.channel_id != competition.channel_id)
            .map(|ctf| {
                CreateButton::new(ctf.channel_id.to_string())
                    .label(format!("Play in {}", &ctf.name))
            })
            .collect();
        join_message
            .edit(
                ctx,
                EditMessage::new().components(if buttons.is_empty() {
                    Vec::new()
                } else {
                    buttons
                        .chunks(5)
                        .map(|b| CreateActionRow::Buttons(b.to_vec()))
                        .collect()
                }),
            )
            .await?;
    }

    // Remove viewing restrictions
    let roles = &ctx
        .guild()
        .ok_or(anyhow::anyhow!("Failed to get roles"))?
        .roles
        .clone();
    let everyone = roles
        .values()
        .find(|role| role.name == "@everyone")
        .ok_or(anyhow::anyhow!("\\@everyone role not found"))?;
    channel
        .delete_permission(ctx, PermissionOverwriteType::Role(everyone.id))
        .await?;

    // Move the channel to the archived category.
    channel
        .edit(ctx, EditChannel::new().category(archived_category_id))
        .await?;

    ctx.say(format!("Archived **{}**.", competition.name))
        .await?;

    Ok(())
}

fn backtick_string(value: &str) -> Result<String, Error> {
    if value.contains("`") {
        Err(anyhow::anyhow!("{value} cannot contain a backtick (`)"))
    } else {
        Ok(format!("`{value}`"))
    }
}

/// Renders the description for forum (shows under some weird button about post guidelines)
fn render_forum_topic(competition: &Competition) -> Result<String, Error> {
    let mut out = format!("**{}**:\n{}\n", competition.name, competition.url);

    for (field_name, value) in competition.login_fields() {
        out.push_str(&format!(
            "\n**{}**: {}",
            field_name,
            backtick_string(value)?
        ));
    }

    Ok(out)
}

/// Renders the embed posted in the general discussion channel
fn render_credentials_embed(competition: &Competition) -> Result<CreateEmbed, Error> {
    let mut out = CreateEmbed::new()
        .color(0xc22026)
        .title(&format!("{} credentials", competition.name))
        .description(competition.url.clone());

    for (field_name, value) in competition.login_fields() {
        out = out.field(field_name, backtick_string(value)?, false);
    }

    Ok(out)
}

// /// Creates a new ctf competition channel.
// #[poise::command(slash_command)]
// pub async fn competition(
//     ctx: CmdContext<'_>,
//     #[description = "Name of the ctf"] name: String,
//     #[description = "Url of ctf website"] url: String,
//     //#[description = "Description of the ctf"] description: Option<String>,
//     #[description = "Team username"] username: String,
//     #[description = "Team password"] password: String,
// ) -> Result<(), Error> {

// }

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
