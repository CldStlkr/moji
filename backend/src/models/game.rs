use crate::db::DbPool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow, Serialize)]
pub struct GameSession {
    pub id: Uuid,
    pub lobby_id: String,
    pub created_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub player_count: i32,
    pub settings: Json<GameSettings>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameSettings {
    pub time_limit: i32,         // Seconds per turn
    pub lives: i32,              // Number of mistakes allowed
    pub kanji_sets: Vec<String>, // N5, N4, etc.
}

#[derive(Debug, FromRow, Serialize)]
pub struct GameAction {
    pub id: i64,
    pub game_id: Uuid,
    pub user_id: Option<Uuid>,
    pub action_type: String,
    pub action_data: Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct PlayerStats {
    pub user_id: Uuid,
    pub username: String,
    pub total_words_submitted: i64,
    pub correct_words: i64,
    pub incorrect_words: i64,
    pub fastest_submission_ms: Option<Option<i32>>,
    pub average_time_ms: Option<Option<i32>>,
}

impl GameSession {
    /// Create a new game session
    pub async fn create(
        pool: &DbPool,
        lobby_id: &str,
        player_count: i32,
        settings: GameSettings,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            GameSession,
            r#"
            INSERT INTO game_sessions (lobby_id, player_count, settings)
            VALUES ($1, $2, $3)
            RETURNING id, lobby_id, created_at, ended_at, player_count, settings as "settings: Json<GameSettings>"
            "#,
            lobby_id,
            player_count,
            Json(settings) as _
        )
        .fetch_one(pool)
        .await
    }

    /// Mark a game session as ended
    pub async fn end_session(pool: &DbPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE game_sessions
            SET ended_at = NOW()
            WHERE id = $1
            "#,
            id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Find a game session by ID
    pub async fn find_by_id(pool: &DbPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            GameSession,
            r#"
            SELECT id, lobby_id, created_at, ended_at, player_count, settings as "settings: Json<GameSettings>"
            FROM game_sessions
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find active game sessions by lobby ID
    pub async fn find_by_lobby(pool: &DbPool, lobby_id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            GameSession,
            r#"
            SELECT id, lobby_id, created_at, ended_at, player_count, settings as "settings: Json<GameSettings>"
            FROM game_sessions
            WHERE lobby_id = $1 AND ended_at IS NULL
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            lobby_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Get recent game sessions
    pub async fn get_recent(pool: &DbPool, limit: i64) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            GameSession,
            r#"
            SELECT id, lobby_id, created_at, ended_at, player_count, settings as "settings: Json<GameSettings>"
            FROM game_sessions
            ORDER BY created_at DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(pool)
        .await
    }
}

impl GameAction {
    /// Record a game action
    pub async fn create(
        pool: &DbPool,
        game_id: Uuid,
        user_id: Option<Uuid>,
        action_type: &str,
        action_data: serde_json::Value,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            GameAction,
            r#"
            INSERT INTO game_actions (game_id, user_id, action_type, action_data)
            VALUES ($1, $2, $3, $4)
            RETURNING id, game_id, user_id, action_type, action_data as "action_data: Json<serde_json::Value>", created_at
            "#,
            game_id,
            user_id,
            action_type,
            Json(action_data) as _
        )
        .fetch_one(pool)
        .await
    }

    /// Get all actions for a game
    pub async fn get_for_game(pool: &DbPool, game_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            GameAction,
            r#"
            SELECT id, game_id, user_id, action_type, action_data as "action_data: Json<serde_json::Value>", created_at
            FROM game_actions
            WHERE game_id = $1
            ORDER BY created_at ASC
            "#,
            game_id
        )
        .fetch_all(pool)
        .await
    }
}

impl PlayerStats {
    /// Get player stats
    pub async fn get_for_user(pool: &DbPool, user_id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            PlayerStats,
            r#"
            SELECT 
                u.id as user_id,
                u.username,
                COUNT(ga.id) as "total_words_submitted!: i64",
                SUM(CASE WHEN ga.action_data->>'correct' = 'true' THEN 1 ELSE 0 END) as "correct_words!: i64",
                SUM(CASE WHEN ga.action_data->>'correct' = 'false' THEN 1 ELSE 0 END) as "incorrect_words!: i64",
                MIN(NULLIF((ga.action_data->>'response_time_ms')::int, 0)) as "fastest_submission_ms: Option<i32>",
                (AVG(NULLIF((ga.action_data->>'response_time_ms')::int, 0)))::int as "average_time_ms: Option<i32>"
            FROM 
                users u
            LEFT JOIN 
                game_actions ga ON ga.user_id = u.id AND ga.action_type = 'word_submission'
            WHERE 
                u.id = $1
            GROUP BY 
                u.id, u.username
            "#,
            user_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Get leaderboard by correct word submissions
    pub async fn get_leaderboard(pool: &DbPool, limit: i64) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            PlayerStats,
            r#"
            SELECT 
                u.id as user_id,
                u.username,
                COUNT(ga.id) as "total_words_submitted!: i64",
                SUM(CASE WHEN ga.action_data->>'correct' = 'true' THEN 1 ELSE 0 END) as "correct_words!: i64",
                SUM(CASE WHEN ga.action_data->>'correct' = 'false' THEN 1 ELSE 0 END) as "incorrect_words!: i64",
                MIN(NULLIF((ga.action_data->>'response_time_ms')::int, 0)) as "fastest_submission_ms: Option<i32>",
                (AVG(NULLIF((ga.action_data->>'response_time_ms')::int, 0)))::int as "average_time_ms: Option<i32>"
            FROM 
                users u
            LEFT JOIN 
                game_actions ga ON ga.user_id = u.id AND ga.action_type = 'word_submission'
            GROUP BY 
                u.id, u.username
            ORDER BY 
                SUM(CASE WHEN ga.action_data->>'correct' = 'true' THEN 1 ELSE 0 END) DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(pool)
        .await
    }
}
