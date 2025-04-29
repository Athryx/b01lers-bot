use std::io::Write;

use serenity::all::{ChannelId, Context, CreateEmbed, CreateMessage};
use tracing::Level;
use tracing_subscriber::{fmt::MakeWriter, FmtSubscriber};

use crate::config::config;

struct ChannelLogger {
    context: Context,
    channel_id: ChannelId,
}

impl ChannelLogger {
    pub fn new(context: Context, channel_id: ChannelId) -> Self {
        ChannelLogger {
            context,
            channel_id,
        }
    }
}

impl MakeWriter<'_> for ChannelLogger {
    type Writer = ChannelWriter;

    fn make_writer(&self) -> Self::Writer {
        ChannelWriter {
            context: self.context.clone(),
            data: Vec::new(),
            channel_id: self.channel_id,
        }
    }
}

struct ChannelWriter {
    context: Context,
    data: Vec<u8>,
    channel_id: ChannelId,
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.data.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let message_content = String::from_utf8_lossy(self.data.as_slice());

        let message_embed = CreateEmbed::new()
            // ansi block allows colorful log messages to display correctly
            .description(format!("```ansi\n{message_content}\n```"))
            .color(0xc22026);

        let message = CreateMessage::new().add_embed(message_embed);

        let channel_id = self.channel_id;
        let context = self.context.clone();

        // FIXME: kind of ugly hack spawning async task for every message to send
        tokio::task::spawn(async move {
            // TODO: handle error when sending message
            let _ = channel_id.send_message(&context, message).await;
        });

        Ok(())
    }
}

impl Drop for ChannelWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

pub fn init_logging(context: Context) {
    let channel_logger = ChannelLogger::new(context, config().server.bot_log_channel);

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_writer(channel_logger)
        .with_ansi(true)
        .pretty()
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting up logging failed");
}
