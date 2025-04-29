use std::collections::HashMap;

use chacha20poly1305::{aead::OsRng, KeyInit, XChaCha20Poly1305};
use serenity::all::{Context, Member, Role, RoleId, User, UserId};
use tracing::info;

use crate::{
    config::config,
    db::{DbConn, DbContext},
    email::EmailClient,
};

pub mod archive;
pub mod bingo;
pub mod challenge;
pub mod competition;
pub mod misc;
pub mod solve;
pub mod stats;
pub mod verify;

pub struct CommandContext {
    pub db: DbContext,
    verify_token_cipher: XChaCha20Poly1305,
    email_client: EmailClient,
}

impl CommandContext {
    pub fn new(db: DbContext, email_client: EmailClient) -> Self {
        let key = XChaCha20Poly1305::generate_key(&mut OsRng);
        CommandContext {
            db,
            verify_token_cipher: XChaCha20Poly1305::new(&key),
            email_client,
        }
    }

    pub async fn conn(&self) -> DbConn {
        self.db.conn().await
    }
}

type Error = anyhow::Error;
type CmdContext<'a> = poise::Context<'a, CommandContext, Error>;

pub async fn get_all_roles(ctx: &Context) -> anyhow::Result<HashMap<RoleId, Role>> {
    let guild_id = config().server.guild_id;

    // TODO: method chain version?
    match ctx.cache.guild(guild_id).map(|g| g.roles.clone()) {
        Some(roles) => Ok(roles),
        None => {
            info!("role cache miss");
            let roles = guild_id.roles(ctx).await?;
            Ok(roles)
        }
    }
}

pub async fn is_officer(ctx: &Context, member: &Member) -> bool {
    let Ok(roles) = get_all_roles(ctx).await else {
        return false;
    };

    member
        .roles
        .iter()
        .map(|role_id| match roles.get(role_id) {
            Some(role) => role.name == config().server.officer_role,
            None => false,
        })
        .fold(false, |a, b| a || b)
}

pub async fn has_perms(ctx: &CmdContext<'_>) -> bool {
    match ctx.author_member().await {
        // make sure a privileged command is being used on b01lers server
        Some(member) => {
            member.guild_id == config().server.guild_id
                && is_officer(ctx.serenity_context(), member.as_ref()).await
        }
        None => false,
    }
}

pub async fn role_id_for_role_name(
    ctx: &Context,
    role_name: &str,
) -> anyhow::Result<Option<RoleId>> {
    let roles = get_all_roles(ctx).await?;

    for (role_id, role) in roles.iter() {
        if role.name == role_name {
            return Ok(Some(*role_id));
        }
    }

    Ok(None)
}

pub async fn user_has_role(ctx: &Context, user: &User, role_name: &str) -> anyhow::Result<bool> {
    let Some(role_id) = role_id_for_role_name(ctx, role_name).await? else {
        // role does not exist, so user does not have role
        return Ok(false);
    };

    let result = user
        .has_role(ctx, config().server.guild_id, role_id)
        .await?;

    Ok(result)
}

/// Adds the given role name to the user in b01lers discord server
pub async fn add_role_to_user(
    ctx: &Context,
    user_id: UserId,
    role_name: &str,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let member_role_id = role_id_for_role_name(ctx, role_name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Role `{role_name}` does not exist"))?;

    let guild_member_id = config()
        .server
        .guild_id
        .member(ctx, user_id)
        .await
        .with_context(|| {
            format!("Could not add role `{role_name}`, you are not in the b01lers discord server")
        })?;

    guild_member_id
        .add_role(ctx, member_role_id)
        .await
        .with_context(|| format!("Could not add role `{role_name}`"))?;

    Ok(())
}

/// Removes the given role name from the user in b01lers discord server
pub async fn remove_role_from_user(
    ctx: &Context,
    user_id: UserId,
    role_name: &str,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let member_role_id = role_id_for_role_name(ctx, role_name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Role `{role_name}` does not exist"))?;

    let guild_member_id = config()
        .server
        .guild_id
        .member(ctx, user_id)
        .await
        .with_context(|| {
            format!("Could not add role `{role_name}`, you are not in the b01lers discord server")
        })?;

    guild_member_id
        .remove_role(ctx, member_role_id)
        .await
        .with_context(|| format!("Could not add role `{role_name}`"))?;

    Ok(())
}
