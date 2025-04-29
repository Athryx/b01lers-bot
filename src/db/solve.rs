use serenity::all::MessageId;
use strum::FromRepr;

#[derive(Debug, Clone)]
pub struct SolveRaw {
    pub id: i64,
    pub challenge_id: i64,
    pub approval_message_id: i64,
    pub flag: String,
    pub approval_status: i64,
}

impl From<Solve> for SolveRaw {
    fn from(value: Solve) -> Self {
        SolveRaw {
            id: value.id,
            challenge_id: value.challenge_id,
            approval_message_id: value.approval_message_id.get() as i64,
            flag: value.flag,
            approval_status: value.approval_status as i64,
        }
    }
}

#[repr(i64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr, strum::Display)]
pub enum ApprovalStatus {
    Pending = 0,
    Approved = 1,
    Declined = 2,
}

#[derive(Debug, Clone)]
pub struct Solve {
    pub id: i64,
    pub challenge_id: i64,
    pub approval_message_id: MessageId,
    pub flag: String,
    pub approval_status: ApprovalStatus,
}

impl From<SolveRaw> for Solve {
    fn from(value: SolveRaw) -> Self {
        Solve {
            id: value.id,
            challenge_id: value.challenge_id,
            approval_message_id: MessageId::new(value.approval_message_id as u64),
            flag: value.flag,
            approval_status: ApprovalStatus::from_repr(value.approval_status)
                .expect("invalid approval status returned from database"),
        }
    }
}
