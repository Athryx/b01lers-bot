mod commands;
mod config;
mod db;
mod email;
mod logging;
mod points;
mod utils;

use clap::Parser;
use dotenvy::dotenv;
use email::EmailClient;
use logging::init_logging;
use points::give_points;
use poise::{BoxFuture, FrameworkContext, FrameworkError};
use serenity::all::{
    Channel, ClientBuilder, Context, CreateMessage, FullEvent, GatewayIntents, Interaction,
};
use std::{env, path::PathBuf};
use tracing::{error, info};

use commands::{CommandContext, restore_user_roles};
use config::config;
use db::DbContext;

/// Runs for every serenity event
///
/// Currently needed for solve approve / reject buttons to work
fn event_handler<'a>(
    context: &'a Context,
    event: &'a FullEvent,
    framework_context: FrameworkContext<'a, CommandContext, anyhow::Error>,
    user_data: &'a CommandContext,
) -> BoxFuture<'a, Result<(), anyhow::Error>> {
    Box::pin(async move {
        match event {
            FullEvent::InteractionCreate {
                interaction: Interaction::Component(component_interaction),
            } => {
                utils::handle_button_interaction(context, user_data, component_interaction).await?
            }
            FullEvent::Message { new_message } => {
                let channel = new_message.channel_id.to_channel(context).await?;

                match channel {
                    Channel::Private(_) => info!(
                        "{} has sent a dm: `{}`",
                        new_message.author.name, new_message.content
                    ),
                    Channel::Guild(channel) if channel.guild_id == config().server.guild_id => {
                        let mut conn = framework_context.user_data().await.conn().await;

                        // give points for sending messages
                        // this also gives points to the bot, this is intentinal
                        give_points(
                            context,
                            &mut conn,
                            new_message.author.id,
                            config().ranks.points_per_message,
                        )
                        .await?;

                        conn.commit().await?;
                    }
                    _ => (),
                }
            }
            FullEvent::GuildMemberAddition { new_member } => {
                if new_member.guild_id == config().server.guild_id {
                    let message = CreateMessage::new().content(&config().server.join_dm_message);

                    new_member.user.direct_message(context, message).await?;

                    if let Ok(user_info) = user_data.conn().await.get_user_by_id(new_member.user.id).await {
                        // give roles if already in server and verified before
                        let roles = restore_user_roles(context, &user_info).await?;
                        if roles.len() > 0 {
                            let role_string = roles.join(", ");
                            let message = CreateMessage::new().content(
                                &format!("restored previous roles to your account on b01lers server: `{role_string}`"),
                            );

                            new_member.user.direct_message(context, message).await?;
                        }
                    }
                }
            }
            _ => (),
        };

        Ok(())
    })
}

fn pre_command_handler<'a>(
    context: poise::Context<'a, CommandContext, anyhow::Error>,
) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        info!("Running command `{}`", context.invocation_string());
    })
}

/// Runs on every error, logs the error in the error channel
fn error_handler<'a>(
    error: FrameworkError<'a, CommandContext, anyhow::Error>,
) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        // first report error in discord channel
        match &error {
            FrameworkError::Command { error, ctx, .. } => {
                error!(
                    "error in `{}` command: {:?}",
                    ctx.invoked_command_name(),
                    error
                );
            }
            FrameworkError::CommandPanic {
                payload: Some(payload),
                ctx,
                ..
            } => {
                error!(
                    "command `{}` has paniced: {}",
                    ctx.invoked_command_name(),
                    payload
                );
            }
            FrameworkError::CommandPanic {
                payload: None, ctx, ..
            } => {
                error!("command `{}` has paniced", ctx.invoked_command_name());
            }
            // TODO: handle other type of errors
            _ => (),
        }

        // then report error to user
        if let Err(e) = poise::builtins::on_error(error).await {
            error!("Error while handling error: {}", e);
        }
    })
}

/// b01lers discord bot
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct CliArgs {
    /// Path to toml config file
    #[arg(long, default_value_t = String::from("config.toml"))]
    config: String,
}

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();

    config::load_config(&PathBuf::from(args.config))
        .await
        .expect("Failed to load config file");

    dotenv().ok();

    let database_url =
        env::var("DATABASE_URL").expect("No `DATABASE_URL` environment variable specified");

    let discord_token =
        env::var("DISCORD_TOKEN").expect("No `DISCORD_TOKEN` environment variable specified");

    let mailgun_token =
        env::var("MAILGUN_TOKEN").expect("No `MAILGUN_TOKEN` environment variable specified");

    let db = DbContext::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::competition::competition(),
                commands::bingo::bingo(),
                commands::archive::archive(),
                commands::challenge::challenge(),
                commands::solve::solve(),
                commands::solve::quick_solve(),
                commands::verify::verify(),
                commands::stats::stats(),
                commands::misc::welcome(),
                commands::misc::get_roles(),
                commands::misc::dm(),
            ],
            event_handler,
            pre_command: pre_command_handler,
            on_error: error_handler,
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                // set up logging to bot log channel
                init_logging(ctx.clone());

                // register the bots commands with discord api on startup
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                // poise::builtins::register_in_guild(
                //     ctx,
                //     &framework.options().commands,
                //     config().server.guild_id,
                // )
                // .await?;

                info!("the bot has logged on");

                let email_client = EmailClient::new(mailgun_token);
                Ok(CommandContext::new(db, email_client))
            })
        })
        .build();

    let intents = GatewayIntents::non_privileged().union(GatewayIntents::GUILD_MEMBERS);
    let mut client = ClientBuilder::new(discord_token, intents)
        .framework(framework)
        .await
        .expect("Failed to create b01lers bot client");

    client.start().await.expect("Failed to run b01lers bot");
}
