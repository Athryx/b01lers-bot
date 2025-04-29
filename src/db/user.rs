use serenity::all::UserId;

use crate::points::Rank;

#[derive(Debug, Clone)]
pub struct UserRaw {
    pub id: i64,
    pub email: Option<String>,
    pub points: i64,
    pub rank: Option<i64>,
}

impl From<User> for UserRaw {
    fn from(value: User) -> Self {
        UserRaw {
            id: value.id.get() as i64,
            email: value.email,
            points: value.points,
            rank: value.rank.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub email: Option<String>,
    pub points: i64,
    pub rank: Rank,
}

impl User {
    pub fn is_verified(&self) -> bool {
        self.email.is_some()
    }
}

impl From<UserRaw> for User {
    fn from(value: UserRaw) -> Self {
        User {
            id: UserId::new(value.id as u64),
            email: value.email,
            points: value.points,
            rank: value.rank.into(),
        }
    }
}
