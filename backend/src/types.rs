use crate::{error::AppError};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use parking_lot::RwLock;
use shared::PlayerId;
use chrono::{DateTime, Utc};

// Newtype wrapper for Arc<RwLock<T>>
#[derive(Clone)]
pub struct Shared<T>(Arc<RwLock<T>>);

impl<T> Shared<T> {
    /// Create a new shared value
    pub fn new(value: T) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }

    /// Execute a closure with mutable access to the shared value
    pub fn write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.0.write();
        f(&mut *guard)
    }

    /// Execute a closure with read-only access to the shared value
    pub fn read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.0.read();
        f(&*guard)
    }

}


#[derive(Clone, Debug)]
pub struct PlayerData {
    pub id: PlayerId,
    pub name: String,
    pub score: u32,
    pub joined_at: DateTime<Utc>,
    pub lives: Option<u32>,
    pub is_eliminated: bool,
    pub is_connected: bool,
}


/// PostgreSQL connection pool
pub type DbPool = Pool<Postgres>;

/// Standard Result type with our AppError
pub type Result<T> = std::result::Result<T, AppError>;
