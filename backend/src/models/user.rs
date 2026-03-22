use crate::db::DbPool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub total_games_played: i32,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub is_guest: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub games_won: i32,
    pub words_guessed_correctly: i32,
    pub total_score: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: Option<String>,
}

impl User {
    /// Create a new user (guest or registered)
    pub async fn create(
        pool: &DbPool,
        username: &str,
        password_hash: Option<String>,
        is_guest: bool
    ) -> Result<Self, sqlx::Error> {
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (username, password_hash, is_guest)
            VALUES ($1, $2, $3) 
            RETURNING id, username, created_at, last_login, total_games_played, password_hash,
                is_guest as "is_guest!: bool",
                last_seen_at, games_won, words_guessed_correctly, total_score
            "#,
            username,
            password_hash,
            is_guest
        )
        .fetch_one(pool)
        .await?;

        let _ = crate::models::GlobalStats::increment_visitors(pool).await;

        Ok(user)
    }

    /// Delete a guest user by username to free up the name
    pub async fn delete_guest_by_username(pool: &DbPool, username: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM users WHERE username = $1 AND is_guest = true",
            username
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Find a user by their ID
    pub async fn find_by_id(pool: &DbPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT id, username, created_at, last_login, total_games_played, password_hash,
                is_guest as "is_guest!: bool",
                last_seen_at, games_won, words_guessed_correctly, total_score
            FROM users 
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find a user by username
    pub async fn find_by_username(
        pool: &DbPool,
        username: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT id, username, created_at, last_login, total_games_played, password_hash,
                is_guest as "is_guest!: bool",
                last_seen_at, games_won, words_guessed_correctly, total_score
            FROM users 
            WHERE username = $1
            "#,
            username
        )
        .fetch_optional(pool)
        .await
    }

    /// Update the user's last login time
    pub async fn update_last_login(pool: &DbPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE users 
            SET last_login = NOW() 
            WHERE id = $1
            "#,
            id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Increment the user's games played count
    pub async fn increment_games_played(pool: &DbPool, id: Uuid) -> Result<i32, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            UPDATE users 
            SET total_games_played = total_games_played + 1 
            WHERE id = $1
            RETURNING total_games_played
            "#,
            id
        )
        .fetch_one(pool)
        .await?;

        Ok(result.total_games_played)
    }

    /// Get top users by games played
    pub async fn get_leaderboard(pool: &DbPool, limit: i64) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT id, username, created_at, last_login, total_games_played, password_hash,
                is_guest as "is_guest!: bool",
                last_seen_at, games_won, words_guessed_correctly, total_score
            FROM users
            ORDER BY total_games_played DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(pool)
        .await
    }

    pub async fn update_last_seen(pool: &DbPool, username: &str) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE users SET last_seen_at = NOW() WHERE username = $1", username)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn update_last_seen_by_id(pool: &DbPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE users SET last_seen_at = NOW() WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn increment_words_guessed(pool: &DbPool, username: &str, score_gained: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE users SET words_guessed_correctly = words_guessed_correctly + 1, total_score = total_score + $1 WHERE username = $2", score_gained, username)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn increment_games_won(pool: &DbPool, username: &str) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE users SET games_won = games_won + 1 WHERE username = $1", username)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_online_count(pool: &DbPool) -> Result<i64, sqlx::Error> {
        let rec = sqlx::query!("SELECT COUNT(*) FROM users WHERE last_seen_at > NOW() - INTERVAL '5 minutes'")
            .fetch_one(pool)
            .await?;
        Ok(rec.count.unwrap_or(0))
    }
}
