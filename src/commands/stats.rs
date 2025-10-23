use poise::CreateReply;
use serenity::all::{CreateEmbed, Mentionable, UserId};
use strum::IntoEnumIterator;

use crate::config::config;
use crate::db::ChallengeType;
use crate::points::{get_point_cutoffs, points_to_string};

use super::{CmdContext, Error};

#[poise::command(slash_command, subcommands("solves", "leaderboard", "rank"))]
pub async fn stats(_ctx: CmdContext<'_>) -> Result<(), Error> {
    Ok(())
}

/// Gets statiscits about the challenges you have solved
#[poise::command(slash_command)]
pub async fn solves(
    ctx: CmdContext<'_>,
    #[description = "User to list stats for (empty to list your own stats)"] user: Option<UserId>,
) -> Result<(), Error> {
    let user_id = match user {
        Some(user_id) => user_id,
        None => ctx.author().id,
    };

    let solves = ctx
        .data()
        .conn()
        .await
        .get_solved_challenges_for_user(user_id)
        .await?;

    let mut stats_embed = CreateEmbed::new()
        .title("CTF Solve Stats")
        .description(format!("Number of challenges {} has solved in each catagory", user_id.mention()))
        .color(0xc22026);

    for category in ChallengeType::iter() {
        let solve_count = solves
            .iter()
            .filter(|solve| solve.category == category)
            .count();

        stats_embed = stats_embed.field(category.to_string(), solve_count.to_string(), true);
    }

    let message = CreateReply::default().embed(stats_embed);

    ctx.send(message).await?;

    Ok(())
}

/// Lists the top point leaders on the server
#[poise::command(slash_command)]
pub async fn leaderboard(ctx: CmdContext<'_>) -> Result<(), Error> {
    let mut embed = CreateEmbed::new()
        .title("b01lers Leaderboard")
        .description("Here is the current leaderboard on the server")
        .color(0xc22026)
        .thumbnail("https://pbs.twimg.com/profile_images/568451513295441921/9Hm60msK_400x400.png");

    let mut users = String::new();
    let mut points = String::new();

    for (i, user) in ctx
        .data()
        .conn()
        .await
        .get_users_by_points(10)
        .await?
        .iter()
        .enumerate()
    {
        let position = match i {
            0 => "🥇".to_string(),
            1 => "🥈".to_string(),
            2 => "🥉".to_string(),
            _ => format!("{}. ", i + 1),
        };

        users.push_str(&format!("{position}{}\n", user.id.mention()));
        points.push_str(&format!("{}\n", points_to_string(user.points)));
    }

    embed = embed
        .field("Users", users, true)
        .field("Points", points, true);

    let message = CreateReply::default().embed(embed);

    ctx.send(message).await?;

    Ok(())
}

/// Lists your points and the point requirements of other ranks
#[poise::command(slash_command)]
pub async fn rank(
    ctx: CmdContext<'_>,
    #[description = "User to list stats for (empty to list your own stats)"] user: Option<UserId>,
) -> Result<(), Error> {
    let user_id = match user {
        Some(user_id) => user_id,
        None => ctx.author().id,
    };

    let mut conn = ctx.data().conn().await;

    let user = conn.get_user_by_id(user_id).await?;

    let mut embed = CreateEmbed::new()
        .title("Server Rank")
        .description("Points can be earned through participation in the server, like sending messages or solving CTF challenges.")
        .color(0xc22026)
        .field("Point Total", points_to_string(user.points), true);

    let cutoffs = get_point_cutoffs(&mut conn).await?;
    let rank_names = &config().ranks.rank_names;
    for (i, (rank, points)) in rank_names.iter().zip(cutoffs).enumerate() {
        embed = embed.field(
            rank,
            format!("Rank #{i} @ {} points.", points_to_string(points)),
            true,
        );
    }

    let message = CreateReply::default().embed(embed);

    ctx.send(message).await?;

    Ok(())
}
