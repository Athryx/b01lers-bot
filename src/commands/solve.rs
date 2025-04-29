use std::collections::HashSet;

use serenity::all::{
    ButtonStyle, ComponentInteraction, ComponentInteractionDataKind, Context, CreateButton,
    CreateEmbed, CreateMessage, EditMessage, EditThread, Mentionable, Message, UserId,
};

use crate::config::config;
use crate::db::{ApprovalStatus, Challenge, ChallengeType, Competition, Solve};
use crate::points::check_rank_up;

use super::{
    competition::{get_challenge_from_ctx, get_competition_from_ctx},
    CmdContext, CommandContext, Error,
};

fn append_solver_ids(solvers: &mut Vec<UserId>, solver_ids: &[Option<UserId>]) {
    for solver_id in solver_ids {
        if let Some(id) = solver_id {
            if !solvers.contains(&id) {
                solvers.push(*id);
            }
        }
    }
}

/// Marks the current channel's challenge as solved
#[poise::command(slash_command)]
pub async fn solve(
    ctx: CmdContext<'_>,
    #[description = "Flag of the challenge that was solved"] flag: String,
    #[description = "First Teammate"] teammate1: Option<UserId>,
    #[description = "Second Teammate"] teammate2: Option<UserId>,
    #[description = "Third Teammate"] teammate3: Option<UserId>,
    #[description = "Fourth Teammate"] teammate4: Option<UserId>,
    #[description = "Fifth Teammate"] teammate5: Option<UserId>,
    #[description = "Sixth Teammate"] teammate6: Option<UserId>,
    #[description = "Seventh Teammate"] teammate7: Option<UserId>,
    #[description = "Eigth Teammate"] teammate8: Option<UserId>,
    #[description = "Ninth Teammate"] teammate9: Option<UserId>,
    #[description = "Tenth Teammate"] teammate10: Option<UserId>,
) -> Result<(), Error> {
    let competition = get_competition_from_ctx(&ctx).await?;
    let challenge = get_challenge_from_ctx(&ctx).await?;

    let mut challenge_channel = ctx
        .guild_channel()
        .await
        .ok_or_else(|| anyhow::anyhow!("You are not inside a challenge channel."))?;

    let competition_forum_channel = challenge_channel
        .parent_id
        .ok_or_else(|| anyhow::anyhow!("You are not inside a challenge channel."))?
        .to_channel(ctx)
        .await
        .or_else(|_| Err(anyhow::anyhow!("You are not inside a challenge channel.")))?
        .guild()
        .ok_or_else(|| anyhow::anyhow!("You are not inside a challenge channel."))?;

    let mut solver_ids = vec![ctx.author().id];
    append_solver_ids(
        &mut solver_ids,
        &[
            teammate1, teammate2, teammate3, teammate4, teammate5, teammate6, teammate7, teammate8,
            teammate9, teammate10,
        ],
    );

    let approval_message =
        send_approval_message(&ctx, &competition, &challenge, &solver_ids, &flag).await?;

    let solve = Solve {
        id: 0,
        challenge_id: challenge.id,
        approval_message_id: approval_message.id,
        flag,
        approval_status: ApprovalStatus::Pending,
    };

    let mut conn = ctx.data().conn().await;

    let solve_id = conn.create_solve(solve, &solver_ids).await?;

    conn.commit().await?;

    // mark challenge channel as solved
    let tag_ids = competition_forum_channel
        .available_tags
        .iter()
        .filter(|t| t.name == challenge.category.to_string() || t.name == "solved")
        .map(|t| t.id);

    challenge_channel
        .edit_thread(ctx, EditThread::new().applied_tags(tag_ids))
        .await?;

    ctx.say(format!(
        "Your solve for {} has been recorded with request ID {solve_id}.",
        challenge.name
    ))
    .await?;

    Ok(())
}

/// Solve a challenge without needing to create a challenge channel
#[poise::command(slash_command)]
pub async fn quick_solve(
    ctx: CmdContext<'_>,
    #[description = "The name of the challenge"] name: String,
    #[description = "Type category of the challenge"] category: ChallengeType,
    #[description = "Flag of the challenge that was solved"] flag: String,
    #[description = "First Teammate"] teammate1: Option<UserId>,
    #[description = "Second Teammate"] teammate2: Option<UserId>,
    #[description = "Third Teammate"] teammate3: Option<UserId>,
    #[description = "Fourth Teammate"] teammate4: Option<UserId>,
    #[description = "Fifth Teammate"] teammate5: Option<UserId>,
    #[description = "Sixth Teammate"] teammate6: Option<UserId>,
    #[description = "Seventh Teammate"] teammate7: Option<UserId>,
    #[description = "Eigth Teammate"] teammate8: Option<UserId>,
    #[description = "Ninth Teammate"] teammate9: Option<UserId>,
    #[description = "Tenth Teammate"] teammate10: Option<UserId>,
) -> Result<(), Error> {
    let competition = get_competition_from_ctx(&ctx).await?;

    let mut solver_ids = vec![ctx.author().id];
    append_solver_ids(
        &mut solver_ids,
        &[
            teammate1, teammate2, teammate3, teammate4, teammate5, teammate6, teammate7, teammate8,
            teammate9, teammate10,
        ],
    );

    let mut conn = ctx.data().conn().await;

    let mut challenge = Challenge {
        id: 0,
        competition_id: competition.channel_id,
        name: name.clone(),
        category,
        channel_id: None,
    };
    challenge.id = conn.create_challenge(challenge.clone()).await?;

    let approval_message =
        send_approval_message(&ctx, &competition, &challenge, &solver_ids, &flag).await?;

    let solve = Solve {
        id: 0,
        challenge_id: challenge.id,
        approval_message_id: approval_message.id,
        flag,
        approval_status: ApprovalStatus::Pending,
    };

    let solve_id = conn.create_solve(solve, &solver_ids).await?;

    conn.commit().await?;

    ctx.say(format!(
        "Your solve for {} has been recorded with request ID {solve_id}.",
        challenge.name
    ))
    .await?;

    Ok(())
}

async fn send_approval_message(
    ctx: &CmdContext<'_>,
    competition: &Competition,
    challenge: &Challenge,
    solver_ids: &[UserId],
    flag: &str,
) -> anyhow::Result<Message> {
    let teammate_string = solver_ids
        .iter()
        .map(|id| id.mention().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let approval_embed = CreateEmbed::new()
        .title(format!("New Solve Request"))
        .description(format!(
            "Here is a new CTF solve request submitted by {}",
            ctx.author().id.mention()
        ))
        .color(0xc22026)
        .thumbnail("https://pbs.twimg.com/profile_images/568451513295441921/9Hm60msK_400x400.png")
        .field("Challenge", &challenge.name, true)
        .field("Category", challenge.category.to_string(), true)
        .field("CTF", competition.channel_id.mention().to_string(), true)
        .field("Flag", format!("```{flag}```"), false)
        .field("Participants", teammate_string, false);

    let accept_button = CreateButton::new("accept")
        .label("Accept")
        .emoji('✅')
        .style(ButtonStyle::Success);

    let reject_button = CreateButton::new("reject")
        .label("Reject")
        .emoji('❎')
        .style(ButtonStyle::Danger);

    let approval_message = CreateMessage::new()
        .add_embed(approval_embed)
        .button(accept_button)
        .button(reject_button);

    let approval_message = config()
        .server
        .solve_approvals_channel_id
        .send_message(ctx, approval_message)
        .await?;

    Ok(approval_message)
}

/// Recieves Component Interaction events and updates solve status if they are an approval button
pub async fn handle_approval_button(
    context: &Context,
    cmd_context: &CommandContext,
    interaction: &ComponentInteraction,
) -> anyhow::Result<()> {
    let mut conn = cmd_context.conn().await;

    if matches!(interaction.data.kind, ComponentInteractionDataKind::Button) {
        let mut message = interaction.message.clone();
        let mut solve = conn.get_solve_by_approval_message_id(message.id).await?;

        if solve.approval_status != ApprovalStatus::Pending {
            message
                .reply(
                    context,
                    format!("solve is alredy {}", solve.approval_status),
                )
                .await?;
        } else if interaction.data.custom_id == "accept" {
            solve.approval_status = ApprovalStatus::Approved;

            // give participants points for solving
            let points_updates = conn
                .give_points_for_solve(solve.id, config().ranks.points_per_solve)
                .await?;

            // rank people up as necassary
            for points_update in points_updates {
                check_rank_up(context, &mut conn, points_update).await?;
            }

            let edit = EditMessage::new()
                .content(format!(
                    "This request is approved by {}",
                    interaction.user.id.mention()
                ))
                .components(Vec::new());

            message.edit(context, edit).await?;
        } else if interaction.data.custom_id == "reject" {
            solve.approval_status = ApprovalStatus::Declined;

            let edit = EditMessage::new()
                .content(format!(
                    "This request is declined by {}",
                    interaction.user.id.mention()
                ))
                .components(Vec::new());

            message.edit(context, edit).await?;
        }

        // save updated approval status
        conn.update_solve(solve).await?;

        conn.commit().await?;

        // acknowledge interaction
        interaction.defer(context).await?;
    }

    Ok(())
}
