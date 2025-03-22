use std::future::Future;
use std::time::Duration;

use serenity::all::{ChannelId, MessageId, UserId};
use sqlx::{sqlite::SqliteConnection, Sqlite};
use sqlx::{TransactionManager, Connection};
use deadpool::{Runtime, managed::{self, Manager, Pool, Timeouts}};

pub use competition::{Competition, BingoSquare};
pub use user::{User, UserRaw};
pub use challenge::{Challenge, ChallengeType};
pub use solve::{ApprovalStatus, Solve};
use competition::CompetitionRaw;
use challenge::ChallengeRaw;
use solve::SolveRaw;

use crate::points::Rank;

mod competition;
mod user;
mod challenge;
mod solve;

pub struct DbContext {
    pool: Pool<ConnectionManager>,
}

struct ConnectionManager {
    url: String,
}

impl Manager for ConnectionManager {
    type Type = SqliteConnection;
    type Error = sqlx::Error;

    fn create(&self) -> impl Future<Output = Result<Self::Type, Self::Error>> + Send {
        SqliteConnection::connect(&self.url)
    }

    async fn recycle(
        &self,
        connection: &mut Self::Type,
        _metrics: &deadpool::managed::Metrics,
    ) -> deadpool::managed::RecycleResult<Self::Error> {
        Ok(connection.ping().await?)
    }
}

struct OutputId { id: i64 }

impl DbContext {
    /// Connects to the database at `url`
    pub async fn connect(url: &str) -> Result<Self, anyhow::Error> {
        // TODO: idk what is a good value for max connections
        let pool = Pool::builder(ConnectionManager { url: String::from(url) })
            .max_size(5)
            .timeouts(Timeouts {
                wait: Some(Duration::from_secs(60)),
                create: Some(Duration::from_secs(10)),
                recycle: Some(Duration::from_secs(10)),
            })
            .runtime(Runtime::Tokio1)
            .build()?;

        Ok(DbContext { pool })
    }

    pub async fn try_conn(&self) -> Result<DbConn, anyhow::Error> {
        let mut connection = self.pool.get().await?;
        connection.begin().await?;
        Ok(DbConn {
            connection,
        })
    }

    pub async fn conn(&self) -> DbConn {
        self.try_conn().await.expect("could not acquire database connection")
    }
}

pub struct DbConn {
    connection: managed::Object<ConnectionManager>,
}

impl DbConn {
    fn connection(& mut self) -> & mut SqliteConnection {
        &mut self.connection
    }

    pub async fn commit(mut self) -> Result<(), anyhow::Error> {
        <Sqlite as sqlx::Database>::TransactionManager::commit(self.connection()).await?;
        Ok(())
    }

    pub async fn rollback(mut self) -> Result<(), anyhow::Error> {
        <Sqlite as sqlx::Database>::TransactionManager::rollback(self.connection()).await?;
        Ok(())
    }

    pub async fn create_competition(&mut self, competition: Competition) -> Result<(), anyhow::Error> {
        let competition_raw: CompetitionRaw = competition.into();
        sqlx::query!(
            "INSERT INTO competition (channel_id, name, bingo) VALUES (?, ?, ?)",
            competition_raw.channel_id,
            competition_raw.name,
            competition_raw.bingo
        )
        .execute(self.connection())
        .await?;

        Ok(())
    }

    pub async fn get_competition(
        &mut self,
        channel_id: ChannelId,
    ) -> Result<Competition, anyhow::Error> {
        let channel_id = channel_id.get() as i64;
        let competition_raw = sqlx::query_as!(
            CompetitionRaw,
            "SELECT * FROM competition WHERE channel_id = ?",
            channel_id
        )
        .fetch_one(self.connection())
        .await?;

        Ok(competition_raw.into())
    }

    pub async fn update_competition(&mut self, competition: Competition) -> Result<(), anyhow::Error> {
        let competition_raw: CompetitionRaw = competition.into();
        sqlx::query!(
            "UPDATE competition SET name = ?, bingo = ? WHERE channel_id = ?",
            competition_raw.name,
            competition_raw.bingo,
            competition_raw.channel_id,
        )
        .execute(self.connection())
        .await?;

        Ok(())
    }

    async fn ensure_user_is_created(&mut self, user_id: UserId) {
        let user_id = user_id.get() as i64;
        // ignore error if user already exists
        let _ = sqlx::query!(
            "INSERT INTO users (id, email, points) VALUES (?, NULL, 0)",
            user_id,
        ).execute(self.connection()).await;
    }

    pub async fn verify_user(&mut self, user_id: UserId, email: &str) -> Result<(), anyhow::Error> {
        self.ensure_user_is_created(user_id).await;

        let user_id = user_id.get() as i64;
        sqlx::query!(
            "UPDATE users SET email = ? WHERE id = ?",
            email,
            user_id,
        ).execute(self.connection()).await?;

        Ok(())
    }

    /// Gives the given user points, creating them if they don't exist
    /// 
    /// # Returns
    /// 
    /// Returns the user's points
    pub async fn give_user_points(&mut self, user_id: UserId, points: i64) -> Result<PointsUpdate, anyhow::Error> {
        self.ensure_user_is_created(user_id).await;

        let user_id = user_id.get() as i64;
        let result = sqlx::query_as!(
            PointsUpdateRaw,
            "UPDATE users SET points = points + ? WHERE id = ? RETURNING id, points, rank",
            points,
            user_id,
        ).fetch_one(self.connection()).await?;

        Ok(PointsUpdate::from_raw(result, points))
    }

    pub async fn set_rank(&mut self, user_id: UserId, rank: Rank) -> Result<(), anyhow::Error> {
        let user_id = user_id.get() as i64;
        let rank: Option<i64> = rank.into();
        sqlx::query!(
            "UPDATE users SET rank = ? WHERE id = ?",
            rank,
            user_id,
        ).execute(self.connection()).await?;

        Ok(())
    }

    pub async fn get_user_by_id(&mut self, id: UserId) -> Result<User, anyhow::Error> {
        let id = id.get() as i64;
        let user_raw = sqlx::query_as!(
            UserRaw,
            "SELECT * FROM users WHERE id = ?",
            id,
        ).fetch_one(self.connection()).await?;

        Ok(user_raw.into())
    }

    pub async fn get_user_by_email(&mut self, email: &str) -> Result<User, anyhow::Error> {
        let user_raw = sqlx::query_as!(
            UserRaw,
            "SELECT * FROM users WHERE email = ?",
            email,
        ).fetch_one(self.connection()).await?;

        Ok(user_raw.into())
    }

    /// Gets the top `count` users with the highest points
    pub async fn get_users_by_points(&mut self, count: u32) -> Result<Vec<User>, anyhow::Error> {
        Ok(sqlx::query_as!(
            UserRaw,
            "SELECT * FROM users ORDER BY points DESC LIMIT ?",
            count,
        ).map(|user| User::from(user))
            .fetch_all(self.connection()).await?)
    }

    /// Creates a new challenge, returning its id
    pub async fn create_challenge(&mut self, challenge: Challenge) -> Result<i64, anyhow::Error> {
        let challenge_raw: ChallengeRaw = challenge.into();

        let id = sqlx::query!(
            "INSERT INTO challenges (competition_id, name, category, channel_id)
            VALUES (?, ?, ?, ?) RETURNING id",
            challenge_raw.competition_id,
            challenge_raw.name,
            challenge_raw.category,
            challenge_raw.channel_id,
        ).fetch_one(self.connection()).await?.id;

        Ok(id)
    }

    pub async fn get_challenge_by_channel_id(&mut self, challenge_id: ChannelId) -> Result<Challenge, anyhow::Error> {
        let challenge_id = challenge_id.get() as i64;

        let challenge = sqlx::query_as!(
            ChallengeRaw,
            "SELECT * FROM challenges WHERE channel_id = ?",
            challenge_id,
        ).fetch_one(self.connection()).await?;

        Ok(challenge.into())
    }

    /// Creates a new solve solved by the given users and returns the solve id
    pub async fn create_solve(&mut self, solve: Solve, users: &[UserId]) -> Result<i64, anyhow::Error> {
        let solve_raw: SolveRaw = solve.into();

        let OutputId { id: solve_id } = sqlx::query_as!(
            OutputId,
            "INSERT INTO solves (challenge_id, approval_message_id, flag, approval_status)
            VALUES (?, ?, ?, ?) RETURNING id",
            solve_raw.challenge_id,
            solve_raw.approval_message_id,
            solve_raw.flag,
            solve_raw.approval_status,
        ).fetch_one(self.connection()).await?;

        for user_id in users {
            let user_id = user_id.get() as i64;

            // ensure user exists first
            // ignore error if user already exists
            let _ = sqlx::query!(
                "INSERT INTO users (id, email, points) VALUES (?, NULL, 0)",
                user_id,
            ).execute(self.connection()).await;

            sqlx::query!(
                "INSERT INTO user_solves (user_id, solve_id) VALUES (?, ?)",
                user_id,
                solve_id,
            ).execute(self.connection()).await?;
        }

        Ok(solve_id)
    }

    pub async fn get_solve_by_approval_message_id(&mut self, message_id: MessageId) -> Result<Solve, anyhow::Error> {
        let id = message_id.get() as i64;
        let solve_raw = sqlx::query_as!(
            SolveRaw,
            "SELECT * FROM solves WHERE approval_message_id = ?",
            id,
        ).fetch_one(self.connection()).await?;

        Ok(solve_raw.into())
    }

    pub async fn get_solved_challenges_for_user(&mut self, user_id: UserId) -> Result<Vec<Challenge>, anyhow::Error> {
        let id = user_id.get() as i64;
        let solves = sqlx::query_as!(
            ChallengeRaw,
            "SELECT challenges.* FROM solves
            INNER JOIN user_solves ON solves.id = user_solves.solve_id
            INNER JOIN challenges ON solves.challenge_id = challenges.id
            WHERE user_solves.user_id = ? AND solves.approval_status = ?",
            id,
            ApprovalStatus::Approved as i64,
        ).map(|solve| Challenge::from(solve))
            .fetch_all(self.connection()).await?;

        Ok(solves)
    }

    /// Updates the flag, and approval status of the given solve
    pub async fn update_solve(&mut self, solve: Solve) -> Result<(), anyhow::Error> {
        let solve_raw: SolveRaw = solve.into();

        sqlx::query!(
            "UPDATE solves SET flag = ?, approval_status = ? WHERE id = ?",
            solve_raw.flag,
            solve_raw.approval_status,
            solve_raw.id,
        ).execute(self.connection()).await?;

        Ok(())
    }

    /// Gives all the participants of this solve some points
    pub async fn give_points_for_solve(&mut self, solve_id: i64, points: i64) -> Result<Vec<PointsUpdate>, anyhow::Error> {
        let result = sqlx::query_as!(
            PointsUpdateRaw,
            "UPDATE users SET points = points + ? WHERE id IN
            (SELECT user_id FROM user_solves WHERE solve_id = ?)
            RETURNING id, points, rank",
            points,
            solve_id,
        ).map(|update| PointsUpdate::from_raw(update, points))
            .fetch_all(self.connection()).await?;

        Ok(result)
    }
}

impl Drop for DbConn {
    fn drop(&mut self) {
        <Sqlite as sqlx::Database>::TransactionManager::start_rollback(self.connection());
    }
}

struct PointsUpdateRaw {
    /// Id of user with changed points
    id: i64,
    /// New points for the user
    points: i64,
    /// Old rank for user
    rank: Option<i64>,
}

/// Represents a change in points that occured in a sql query
#[derive(Debug)]
pub struct PointsUpdate {
    pub user_id: UserId,
    pub old_points: i64,
    pub new_points: i64,
    pub old_rank: Rank,
}

impl PointsUpdate {
    fn from_raw(update: PointsUpdateRaw, point_increase: i64) -> Self {
        PointsUpdate {
            user_id: UserId::new(update.id as u64),
            old_points: update.points - point_increase,
            new_points: update.points,
            old_rank: update.rank.into(),
        }
    }
}