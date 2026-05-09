//! Database access layer.
//!
//! All PostgreSQL interaction is funnelled through this module. Functions return
//! `Result<_, DbError>` so callers can handle errors uniformly.

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::{NoTls, error::SqlState};
use std::time::{SystemTime, UNIX_EPOCH};

/// A registered user as stored in the database.
#[derive(Clone, Debug)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: i64,
    pub email_verified: bool,
}

/// Aggregated game statistics for a user's public profile.
pub struct UserStats {
    pub total: i64,
    pub wins: i64,
    pub losses: i64,
    pub draws: i64,
}

/// A condensed game entry returned by [`get_user_games`].
pub struct GameSummary {
    pub id: i64,
    pub game_id: String,
    pub room_code: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub result: Option<String>,
    pub outcome: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("connection pool error: {0}")]
    Pool(#[from] deadpool_postgres::PoolError),
    #[error("database error: {0}")]
    Db(#[from] tokio_postgres::Error),
}

impl DbError {
    pub fn is_unique_violation(&self) -> bool {
        if let DbError::Db(e) = self {
            e.code() == Some(&SqlState::UNIQUE_VIOLATION)
        } else {
            false
        }
    }
}

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Connects to the PostgreSQL database at `url` and runs all pending migrations.
pub async fn init_db(url: &str) -> Pool {
    let pg_config: tokio_postgres::Config = url.parse().expect("Invalid DATABASE_URL");
    let manager = Manager::from_config(
        pg_config,
        NoTls,
        ManagerConfig { recycling_method: RecyclingMethod::Fast },
    );
    let pool = Pool::builder(manager)
        .max_size(5)
        .build()
        .expect("Failed to build connection pool");

    let client = pool.get().await.expect("Failed to get connection for migrations");
    client
        .batch_execute(include_str!("../migrations/001_init.sql"))
        .await
        .expect("Migration 001 failed");
    client
        .batch_execute(include_str!("../migrations/002_participants_unique.sql"))
        .await
        .expect("Migration 002 failed");
    client
        .batch_execute(include_str!("../migrations/003_email_verification.sql"))
        .await
        .expect("Migration 003 failed");

    pool
}

// ── Users ────────────────────────────────────────────────────────────────────

fn user_from_row(r: &tokio_postgres::Row) -> User {
    User {
        id: r.get("id"),
        username: r.get("username"),
        email: r.get("email"),
        password_hash: r.get("password_hash"),
        created_at: r.get("created_at"),
        email_verified: r.get("email_verified"),
    }
}

pub async fn create_user(
    pool: &Pool,
    username: &str,
    email: &str,
    password_hash: &str,
) -> Result<i64, DbError> {
    let client = pool.get().await?;
    let row = client
        .query_one(
            "INSERT INTO users (username, email, password_hash, created_at, email_verified) \
             VALUES ($1, $2, $3, $4, FALSE) RETURNING id",
            &[&username, &email, &password_hash, &now_unix()],
        )
        .await?;
    Ok(row.get(0))
}

pub async fn get_user_by_id(pool: &Pool, id: i64) -> Result<Option<User>, DbError> {
    let client = pool.get().await?;
    let row = client
        .query_opt(
            "SELECT id, username, email, password_hash, created_at, email_verified \
             FROM users WHERE id = $1",
            &[&id],
        )
        .await?;
    Ok(row.as_ref().map(user_from_row))
}

pub async fn get_user_by_username(pool: &Pool, username: &str) -> Result<Option<User>, DbError> {
    let client = pool.get().await?;
    let row = client
        .query_opt(
            "SELECT id, username, email, password_hash, created_at, email_verified \
             FROM users WHERE username = $1",
            &[&username],
        )
        .await?;
    Ok(row.as_ref().map(user_from_row))
}

pub async fn get_user_by_email(pool: &Pool, email: &str) -> Result<Option<User>, DbError> {
    let client = pool.get().await?;
    let row = client
        .query_opt(
            "SELECT id, username, email, password_hash, created_at, email_verified \
             FROM users WHERE email = $1",
            &[&email],
        )
        .await?;
    Ok(row.as_ref().map(user_from_row))
}

/// Looks up a user by username first; if not found, tries by email.
pub async fn get_user_by_username_or_email(pool: &Pool, login: &str) -> Result<Option<User>, DbError> {
    if let Some(u) = get_user_by_username(pool, login).await? {
        return Ok(Some(u));
    }
    get_user_by_email(pool, login).await
}

pub async fn set_email_verified(pool: &Pool, user_id: i64) -> Result<(), DbError> {
    let client = pool.get().await?;
    client
        .execute(
            "UPDATE users SET email_verified = TRUE WHERE id = $1",
            &[&user_id],
        )
        .await?;
    Ok(())
}

pub async fn update_password_hash(pool: &Pool, user_id: i64, hash: &str) -> Result<(), DbError> {
    let client = pool.get().await?;
    client
        .execute(
            "UPDATE users SET password_hash = $1 WHERE id = $2",
            &[&hash, &user_id],
        )
        .await?;
    Ok(())
}

// ── Email tokens ──────────────────────────────────────────────────────────────

pub async fn create_email_token(
    pool: &Pool,
    user_id: i64,
    token: &str,
    kind: &str,
    expires_at: i64,
) -> Result<(), DbError> {
    let client = pool.get().await?;
    client
        .execute(
            "INSERT INTO email_tokens (user_id, token, kind, expires_at, created_at) \
             VALUES ($1, $2, $3, $4, $5)",
            &[&user_id, &token, &kind, &expires_at, &now_unix()],
        )
        .await?;
    Ok(())
}

/// Removes all tokens of the given kind for a user (call before creating a fresh one).
pub async fn delete_email_tokens(pool: &Pool, user_id: i64, kind: &str) -> Result<(), DbError> {
    let client = pool.get().await?;
    client
        .execute(
            "DELETE FROM email_tokens WHERE user_id = $1 AND kind = $2",
            &[&user_id, &kind],
        )
        .await?;
    Ok(())
}

/// Atomically deletes the token row and returns the `user_id` if the token
/// exists and has not expired. Returns `None` for missing or expired tokens.
pub async fn consume_email_token(
    pool: &Pool,
    token: &str,
    kind: &str,
) -> Result<Option<i64>, DbError> {
    let client = pool.get().await?;
    let row = client
        .query_opt(
            "DELETE FROM email_tokens WHERE token = $1 AND kind = $2 \
             RETURNING user_id, expires_at",
            &[&token, &kind],
        )
        .await?;

    Ok(row.and_then(|r| {
        let expires_at: i64 = r.get("expires_at");
        if expires_at >= now_unix() {
            Some(r.get("user_id"))
        } else {
            None
        }
    }))
}

// ── Game records ─────────────────────────────────────────────────────────────

/// Creates a new game record when a room opens. Returns the record id.
pub async fn insert_game_record(
    pool: &Pool,
    game_id: &str,
    room_code: &str,
) -> Result<i64, DbError> {
    let client = pool.get().await?;
    let row = client
        .query_one(
            "INSERT INTO game_records (game_id, room_code, started_at) \
             VALUES ($1, $2, $3) RETURNING id",
            &[&game_id, &room_code, &now_unix()],
        )
        .await?;
    Ok(row.get(0))
}

/// Stamps `ended_at` and stores the opaque result JSON supplied by the game.
pub async fn close_game_record(
    pool: &Pool,
    record_id: i64,
    result_json: Option<&str>,
) -> Result<(), DbError> {
    // AND ended_at IS NULL prevents overwriting a result already set by POST /games/result
    let client = pool.get().await?;
    client
        .execute(
            "UPDATE game_records SET ended_at = $1, result = $2 \
             WHERE id = $3 AND ended_at IS NULL",
            &[&now_unix(), &result_json, &record_id],
        )
        .await?;
    Ok(())
}

/// Records a player's participation in a game. `user_id` is `None` for anonymous players.
pub async fn insert_participant(
    pool: &Pool,
    record_id: i64,
    user_id: Option<i64>,
    player_id: u16,
    outcome: Option<&str>,
) -> Result<(), DbError> {
    let client = pool.get().await?;
    client
        .execute(
            "INSERT INTO game_participants (game_record_id, user_id, player_id, outcome) \
             VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
            &[&record_id, &user_id, &(player_id as i64), &outcome],
        )
        .await?;
    Ok(())
}

/// Returns win/loss/draw counts for a user. All values are 0 when the user has no games.
pub async fn get_user_stats(pool: &Pool, user_id: i64) -> Result<UserStats, DbError> {
    let client = pool.get().await?;
    let row = client
        .query_one(
            "SELECT
                 COUNT(*) as total,
                 COALESCE(SUM(CASE WHEN outcome = 'win'  THEN 1 ELSE 0 END), 0::BIGINT) as wins,
                 COALESCE(SUM(CASE WHEN outcome = 'loss' THEN 1 ELSE 0 END), 0::BIGINT) as losses,
                 COALESCE(SUM(CASE WHEN outcome = 'draw' THEN 1 ELSE 0 END), 0::BIGINT) as draws
             FROM game_participants
             WHERE user_id = $1",
            &[&user_id],
        )
        .await?;
    Ok(UserStats {
        total: row.get("total"),
        wins: row.get("wins"),
        losses: row.get("losses"),
        draws: row.get("draws"),
    })
}

/// Returns a paginated list of games a user participated in, newest first.
pub async fn get_user_games(
    pool: &Pool,
    user_id: i64,
    page: i64,
    per_page: i64,
) -> Result<Vec<GameSummary>, DbError> {
    let client = pool.get().await?;
    let rows = client
        .query(
            "SELECT gr.id, gr.game_id, gr.room_code, gr.started_at, gr.ended_at, gr.result, gp.outcome
             FROM game_records gr
             JOIN game_participants gp ON gp.game_record_id = gr.id
             WHERE gp.user_id = $1
             ORDER BY gr.started_at DESC
             LIMIT $2 OFFSET $3",
            &[&user_id, &per_page, &(page * per_page)],
        )
        .await?;
    Ok(rows
        .into_iter()
        .map(|r| GameSummary {
            id: r.get("id"),
            game_id: r.get("game_id"),
            room_code: r.get("room_code"),
            started_at: r.get("started_at"),
            ended_at: r.get("ended_at"),
            result: r.get("result"),
            outcome: r.get("outcome"),
        })
        .collect())
}
