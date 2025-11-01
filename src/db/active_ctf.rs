use serenity::all::{ChannelId, MessageId};

#[derive(Debug, Clone)]
pub struct ActiveCtfRaw {
    pub join_id: i64,
    pub channel_id: i64,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ActiveCtf {
    pub join_id: MessageId,
    pub channel_id: ChannelId,
    pub name: String,
}

impl From<ActiveCtf> for ActiveCtfRaw {
    fn from(value: ActiveCtf) -> Self {
        ActiveCtfRaw {
            join_id: value.join_id.get() as i64,
            channel_id: value.channel_id.get() as i64,
            name: value.name,
        }
    }
}

impl From<ActiveCtfRaw> for ActiveCtf {
    fn from(value: ActiveCtfRaw) -> Self {
        ActiveCtf {
            join_id: (value.join_id as u64).into(),
            channel_id: (value.channel_id as u64).into(),
            name: value.name,
        }
    }
}
