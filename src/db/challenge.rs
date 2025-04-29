use poise::macros::ChoiceParameter;
use serenity::all::ChannelId;
use strum::FromRepr;

#[derive(Debug, Clone)]
pub struct ChallengeRaw {
    pub id: i64,
    pub competition_id: i64,
    pub name: String,
    pub category: i64,
    pub channel_id: Option<i64>,
}

impl From<Challenge> for ChallengeRaw {
    fn from(value: Challenge) -> Self {
        ChallengeRaw {
            id: value.id,
            competition_id: value.competition_id.get() as i64,
            name: value.name,
            category: value.category as i64,
            channel_id: value.channel_id.map(|id| id.get() as i64),
        }
    }
}

#[repr(i64)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    ChoiceParameter,
    FromRepr,
    strum::Display,
    strum::EnumIter,
)]
pub enum ChallengeType {
    #[name = "rev"]
    #[strum(to_string = "rev")]
    Rev = 0,
    #[name = "pwn"]
    #[strum(to_string = "pwn")]
    Pwn = 1,
    #[name = "web"]
    #[strum(to_string = "web")]
    Web = 2,
    #[name = "crypto"]
    #[strum(to_string = "crypto")]
    Crypto = 3,
    #[name = "misc"]
    #[strum(to_string = "misc")]
    Misc = 4,
    #[name = "osint"]
    #[strum(to_string = "osint")]
    Osint = 5,
    #[name = "forensics"]
    #[strum(to_string = "forensics")]
    Forensics = 6,
    #[name = "blockchain"]
    #[strum(to_string = "blockchain")]
    Blockchain = 7,
    #[name = "programming"]
    #[strum(to_string = "programming")]
    Programming = 8,
    #[name = "jail"]
    #[strum(to_string = "jail")]
    Jail = 9,
    #[name = "welcome"]
    #[strum(to_string = "welcome")]
    Welcome = 10,
}

#[derive(Debug, Clone)]
pub struct Challenge {
    pub id: i64,
    pub competition_id: ChannelId,
    pub name: String,
    pub category: ChallengeType,
    pub channel_id: Option<ChannelId>,
}

impl From<ChallengeRaw> for Challenge {
    fn from(value: ChallengeRaw) -> Self {
        Challenge {
            id: value.id,
            competition_id: ChannelId::new(value.competition_id as u64),
            name: value.name,
            category: ChallengeType::from_repr(value.category)
                .expect("invalid challenge category returned from database"),
            channel_id: value.channel_id.map(|id| ChannelId::new(id as u64)),
        }
    }
}
