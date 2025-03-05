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
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
}

impl User {
    /// Create a new user in the database
    pub async fn create(pool: &DbPool, username: &str) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (username) 
            VALUES ($1) 
            RETURNING id, username, created_at, last_login, total_games_played
            "#,
            username
        )
        .fetch_one(pool)
        .await
    }

    /// Find a user by their ID
    pub async fn find_by_id(pool: &DbPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT id, username, created_at, last_login, total_games_played 
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
            SELECT id, username, created_at, last_login, total_games_played 
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
            SELECT id, username, created_at, last_login, total_games_played 
            FROM users 
            ORDER BY total_games_played DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(pool)
        .await
    }
}
