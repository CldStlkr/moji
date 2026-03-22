use crate::db::DbPool;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize, Clone)]
pub struct GlobalStats {
    pub total_unique_visitors: i64,
    pub total_games_played: i64,
    pub total_words_guessed: i64,
    pub peak_concurrent_players: i32,
    pub updated_at: DateTime<Utc>,
}

impl GlobalStats {
    pub async fn get(pool: &DbPool) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            GlobalStats,
            "SELECT total_unique_visitors, total_games_played, total_words_guessed, peak_concurrent_players, updated_at FROM global_stats WHERE id = 1"
        )
        .fetch_one(pool)
        .await
    }

    pub async fn increment_games(pool: &DbPool) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE global_stats SET total_games_played = total_games_played + 1, updated_at = NOW() WHERE id = 1")
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn increment_words(pool: &DbPool) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE global_stats SET total_words_guessed = total_words_guessed + 1, updated_at = NOW() WHERE id = 1")
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn increment_visitors(pool: &DbPool) -> Result<(), sqlx::Error> {
        sqlx::query!("UPDATE global_stats SET total_unique_visitors = total_unique_visitors + 1, updated_at = NOW() WHERE id = 1")
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn update_peak_players(pool: &DbPool, current: i32) -> Result<(), sqlx::Error> {
         sqlx::query!("UPDATE global_stats SET peak_concurrent_players = GREATEST(peak_concurrent_players, $1), updated_at = NOW() WHERE id = 1", current)
            .execute(pool)
            .await?;
         Ok(())
    }
}
