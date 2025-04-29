use crate::{
    commands::{add_role_to_user, remove_role_from_user},
    config::config,
    db::{DbConn, PointsUpdate},
};

use serenity::all::{Context, CreateMessage, UserId};

pub async fn get_point_cutoffs(db: &mut DbConn) -> anyhow::Result<Vec<i64>> {
    let mut max_score = db.get_users_by_points(1).await?.first().unwrap().points;

    let rank_count = config().ranks.rank_count();

    let mut cutoffs = vec![0; rank_count];

    for i in (0..rank_count).rev() {
        cutoffs[i] = max_score;
        max_score = (max_score as f64 * 0.75) as i64;
    }

    Ok(cutoffs)
}

struct RankManager {
    point_cutoffs: Vec<i64>,
}

impl RankManager {
    async fn new(db: &mut DbConn) -> anyhow::Result<Self> {
        Ok(RankManager {
            point_cutoffs: get_point_cutoffs(db).await?,
        })
    }

    fn rank_for_points(&self, points: i64) -> Rank {
        match self.point_cutoffs.binary_search(&points) {
            Ok(i) => Rank::Rank(i),
            Err(i) => {
                if i == 0 {
                    // the points is less than the lowest rank
                    Rank::Unranked
                } else {
                    Rank::Rank(i - 1)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    Unranked,
    Rank(usize),
}

impl Rank {
    pub fn rank_name(&self) -> Option<&'static str> {
        match self {
            Self::Unranked => None,
            Self::Rank(i) => Some(&config().ranks.rank_names[*i]),
        }
    }
}

impl From<Option<i64>> for Rank {
    fn from(value: Option<i64>) -> Self {
        match value {
            None => Self::Unranked,
            Some(i) => Self::Rank(i as usize),
        }
    }
}

impl Into<Option<i64>> for Rank {
    fn into(self) -> Option<i64> {
        match self {
            Self::Unranked => None,
            Self::Rank(i) => Some(i as i64),
        }
    }
}

pub async fn check_rank_up(
    context: &Context,
    db: &mut DbConn,
    points_update: PointsUpdate,
) -> anyhow::Result<()> {
    let rank_manager = RankManager::new(db).await?;

    // check max of current old points and rank, they might have earned a rank in the past
    let old_rank = std::cmp::max(
        rank_manager.rank_for_points(points_update.old_points),
        points_update.old_rank,
    );
    let new_rank = rank_manager.rank_for_points(points_update.new_points);

    if new_rank > old_rank {
        let user = points_update.user_id.to_user(context).await?;

        if let Some(new_rank_name) = new_rank.rank_name() {
            add_role_to_user(context, points_update.user_id, new_rank_name).await?;

            // create rank up message
            let message = CreateMessage::new().content(format!(
                "{} has reached the rank {}!",
                user.name, new_rank_name
            ));

            config()
                .server
                .rank_up_channel
                .send_message(context, message)
                .await?;
        }

        db.set_rank(points_update.user_id, new_rank).await?;

        if let Some(old_rank_name) = old_rank.rank_name() {
            remove_role_from_user(context, points_update.user_id, old_rank_name).await?;
        }
    }

    Ok(())
}

pub async fn give_points(
    context: &Context,
    db: &mut DbConn,
    user_id: UserId,
    points: i64,
) -> anyhow::Result<()> {
    let points_update = db.give_user_points(user_id, points).await?;

    check_rank_up(context, db, points_update).await?;

    Ok(())
}

/// Converts points to a string to be displayed
///
/// Points are displayed factor of 10 less with a decimal place
pub fn points_to_string(points: i64) -> String {
    format!("{}.{}", points / 10, points.abs() % 10)
}
