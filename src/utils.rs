use serenity::all::{
    ChannelId, ComponentInteraction, ComponentInteractionDataKind, Context, PermissionOverwrite,
    PermissionOverwriteType, Permissions,
};

use crate::{commands::CommandContext, config::config};

pub async fn handle_button_interaction(
    context: &Context,
    cmd_context: &CommandContext,
    interaction: &ComponentInteraction,
) -> anyhow::Result<()> {
    let approvals_id = config().server.solve_approvals_channel_id;
    let join_id = config().server.ctf_join_channel;

    if !matches!(interaction.data.kind, ComponentInteractionDataKind::Button) {
        return Err(anyhow::anyhow!(
            "Called handle_button_interaction without a button"
        ));
    }
    match interaction.channel_id {
        id if id == approvals_id => {
            crate::commands::solve::handle_approval_button(context, cmd_context, interaction)
                .await?
        }

        id if id == join_id => handle_ctf_join(context, cmd_context, interaction).await?,
        _ => return Err(anyhow::anyhow!("Unknown usage of button")),
    }
    Ok(())
}

pub async fn handle_ctf_join(
    context: &Context,
    cmd_context: &CommandContext,
    interaction: &ComponentInteraction,
) -> anyhow::Result<()> {
    let channel_id = ChannelId::new(interaction.data.custom_id.parse()?);
    let member = interaction
        .member
        .as_ref()
        .ok_or(anyhow::anyhow!("No member for button press"))?;
    let member_id = member.user.id;

    let guild = interaction
        .guild_id
        .ok_or(anyhow::anyhow!("No guild found"))?;
    let channels = guild.channels(context).await?;

    let ctf = channels
        .get(&channel_id)
        .ok_or(anyhow::anyhow!("CTF not found in channel"))?;

    ctf.create_permission(
        context,
        PermissionOverwrite {
            kind: PermissionOverwriteType::Member(member_id),
            allow: Permissions::VIEW_CHANNEL
                | Permissions::READ_MESSAGE_HISTORY
                | Permissions::SEND_MESSAGES
                | Permissions::ATTACH_FILES,
            deny: Permissions::empty(),
        },
    )
    .await?;

    cmd_context
        .conn()
        .await
        .add_ctf_participant(member_id, channel_id)
        .await?;
    interaction.defer(context).await?;

    Ok(())
}
