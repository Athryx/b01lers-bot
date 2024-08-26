use std::{path::Path, sync::OnceLock};

use tokio::fs::read_to_string;
use serde::{Serialize, Deserialize};
use serenity::all::{GuildId, ChannelId};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub mailgun: MailgunConfig,
    pub server: ServerConfig,
    pub ranks: RankConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MailgunConfig {
    pub api_base_url: String,
    pub email_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub guild_id: GuildId,
    /// channel group with active ctf channels
    pub ctf_category_id: ChannelId,
    pub archived_ctf_category_id: ChannelId,
    pub solve_approvals_channel_id: ChannelId,
    pub bot_log_channel: ChannelId,
    pub rank_up_channel: ChannelId,
    pub officer_role: String,
    pub member_role: String,
    pub welcome_message: String,
    /// Message sent to people when they join the server
    pub join_dm_message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RankConfig {
    pub points_per_solve: i64,
    pub points_per_message: i64,
    pub rank_names: Vec<String>,
}

impl RankConfig {
    pub fn rank_count(&self) -> usize {
        self.rank_names.len()
    }
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub async fn load_config(path: &Path) -> anyhow::Result<()> {
    let config_data = read_to_string(path).await?;
    let config = toml::from_str(&config_data)?;
    CONFIG.set(config)
        .or_else(|_| Err(anyhow::anyhow!("config already loaded")))?;

    Ok(())
}

pub fn config() -> &'static Config {
    CONFIG.get().expect("config not loaded yet")
}