use crate::error::AppError;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use parking_lot::RwLock;
use shared::PlayerId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Newtype wrapper for Arc<RwLock<T>> — kept for any remaining non-lobby uses
#[derive(Clone)]
pub struct Shared<T>(Arc<RwLock<T>>);

impl<T> Shared<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }

    pub fn write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.0.write();
        f(&mut *guard)
    }

    pub fn read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.0.read();
        f(&*guard)
    }
}

// Serialize/Deserialize required so PlayerData can be stored as JSON in Redis.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerData {
    pub id: PlayerId,
    pub name: String,
    pub score: u32,
    pub joined_at: DateTime<Utc>,
    pub lives: Option<u32>,
    pub is_eliminated: bool,
    pub is_connected: bool,
    pub is_spectator: bool,
}


/// PostgreSQL connection pool
pub type DbPool = Pool<Postgres>;

/// Standard Result type with custom AppError
pub type Result<T> = std::result::Result<T, AppError>;
