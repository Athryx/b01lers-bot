use std::cmp::min;
use std::time::Duration;

use poise::CreateReply;
use serenity::{
    all::{
        ComponentInteractionCollector, CreateActionRow, CreateButton, CreateEmbed,
        CreateInteractionResponse, CreateInteractionResponseMessage,
    },
    builder::CreateEmbedFooter,
    model::application::ButtonStyle,
};

use super::CmdContext;

// how long pagination buttons are active for after sending message
const PAGINATION_TIMEOUT: Duration = Duration::from_mins(3);

// copied from poise example pagination
pub async fn send_pagination<T>(
    ctx: CmdContext<'_>,
    mut render: impl FnMut(usize, &[T]) -> CreateEmbed,
    data: &[T],
    items_per_page: usize,
) -> anyhow::Result<()> {
    let mut current_page = 0;
    let last_page = (data.len().saturating_sub(1)) / items_per_page;

    let mut render_embed = |current_page| {
        let render_data = &data
            [items_per_page * current_page..min(items_per_page * (current_page + 1), data.len())];

        render(current_page, render_data).footer(CreateEmbedFooter::new(format!(
            "Page {} of {}",
            current_page + 1,
            last_page + 1,
        )))
    };

    if last_page == 0 {
        // it all fits in one page, don't paginate
        let reply = CreateReply::default().embed(render_embed(0));

        ctx.send(reply).await?;
        return Ok(());
    }

    // Define some unique identifiers for the navigation buttons
    let ctx_id = ctx.id();
    let start_button_id = format!("{}start", ctx_id);
    let prev_button_id = format!("{}prev", ctx_id);
    let next_button_id = format!("{}next", ctx_id);
    let end_button_id = format!("{}end", ctx_id);

    let buttons = vec![
        CreateButton::new(&start_button_id)
            .label("◀◀")
            .style(ButtonStyle::Secondary),
        CreateButton::new(&prev_button_id)
            .label("◀")
            .style(ButtonStyle::Secondary),
        CreateButton::new(&next_button_id)
            .label("▶")
            .style(ButtonStyle::Secondary),
        CreateButton::new(&end_button_id)
            .label("▶▶")
            .style(ButtonStyle::Secondary),
    ];
    let disabled_buttons = buttons
        .iter()
        .map(|button| button.clone().disabled(true))
        .collect::<Vec<_>>();

    // Send the embed with the first page as content
    let reply = {
        let components = CreateActionRow::Buttons(buttons);

        CreateReply::default()
            .embed(render_embed(0))
            .components(vec![components])
    };

    let reply_handle = ctx.send(reply).await?;

    // Loop through incoming interactions with the navigation buttons
    while let Some(press) = ComponentInteractionCollector::new(ctx)
        // We defined our button IDs to start with `ctx_id`. If they don't, some other command's
        // button was pressed
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        // Timeout when no navigation button has been pressed for 24 hours
        .timeout(PAGINATION_TIMEOUT)
        .await
    {
        // Depending on which button was pressed, go to next or previous page
        if press.data.custom_id == next_button_id {
            current_page += 1;
            if current_page > last_page {
                current_page = 0;
            }
        } else if press.data.custom_id == prev_button_id {
            current_page = current_page.checked_sub(1).unwrap_or(last_page);
        } else if press.data.custom_id == start_button_id {
            current_page = 0;
        } else if press.data.custom_id == end_button_id {
            current_page = last_page;
        } else {
            // This is an unrelated button interaction
            continue;
        }

        // Update the message with the new page contents
        press
            .create_response(
                ctx.serenity_context(),
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new().embed(render_embed(current_page)),
                ),
            )
            .await?;
    }

    // disable buttons
    reply_handle
        .edit(
            ctx,
            CreateReply::default()
                .embed(render_embed(0))
                .components(vec![CreateActionRow::Buttons(disabled_buttons)]),
        )
        .await?;

    Ok(())
}
