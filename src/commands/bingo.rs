use poise::CreateReply;
use serenity::all::CreateAttachment;

use crate::commands::competition::get_competition_from_ctx;
use crate::commands::{CmdContext, Error};
use crate::db::BingoSquare;

async fn send_bingo_image(ctx: &CmdContext<'_>, image: &[u8]) -> Result<(), Error> {
    let attachment = CreateAttachment::bytes(image, "bingo_squares.png");
    let reply = CreateReply::default().attachment(attachment);

    ctx.send(reply).await?;
    Ok(())
}

// This can never be called, just needed for bingo subcommands
#[poise::command(slash_command, subcommands("add", "status", "remove"))]
pub async fn bingo(_ctx: CmdContext<'_>) -> Result<(), Error> {
    Ok(())
}

/// Displays the current status of the bad ctf bingo squares
#[poise::command(slash_command)]
pub async fn status(ctx: CmdContext<'_>) -> Result<(), Error> {
    let competition = get_competition_from_ctx(&ctx).await?;

    send_bingo_image(&ctx, &competition.get_bingo_picture_png_bytes()?).await?;

    Ok(())
}

/// Checks off a bad ctf bingo square
#[poise::command(slash_command)]
pub async fn add(
    ctx: CmdContext<'_>,
    #[description = "The bingo square to check off"] square: BingoSquare,
) -> Result<(), Error> {
    let mut competition = get_competition_from_ctx(&ctx).await?;
    competition.bingo |= square;

    let mut conn = ctx.data().conn().await;

    send_bingo_image(&ctx, &competition.get_bingo_picture_png_bytes()?).await?;
    conn.update_competition(competition).await?;

    conn.commit().await?;

    Ok(())
}

/// Unmarks a bad ctf bingo square
#[poise::command(slash_command)]
pub async fn remove(
    ctx: CmdContext<'_>,
    #[description = "The bingo square to uncheck"] square: BingoSquare,
) -> Result<(), Error> {
    let mut competition = get_competition_from_ctx(&ctx).await?;
    competition.bingo.remove(square);

    let mut conn = ctx.data().conn().await;

    send_bingo_image(&ctx, &competition.get_bingo_picture_png_bytes()?).await?;
    conn.update_competition(competition).await?;

    conn.commit().await?;

    Ok(())
}
