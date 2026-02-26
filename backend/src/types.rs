use crate::{error::AppError, LobbyState};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use parking_lot::RwLock;

// Newtype wrapper for Arc<RwLock<T>>
#[derive(Clone)]
pub struct Shared<T>(Arc<RwLock<T>>);

impl<T> Shared<T> {
    /// Create a new shared value
    pub fn new(value: T) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }

    /// Execute a closure with mutable access to the shared value
    /// Automatically converts poison errors to AppError
    pub fn write<F, R>(&self, f: F) -> crate::types::Result<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.0.write();
        Ok(f(&mut *guard))
    }

    /// Execute a closure with read-only access to the shared value
    /// Automatically converts poison errors to AppError
    pub fn read<F, R>(&self, f: F) -> crate::types::Result<R>
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.0.read();
        Ok(f(&*guard))
    }

}



/// Shared lobby state reference
pub type SharedState = Arc<LobbyState>;

/// PostgreSQL connection pool
pub type DbPool = Pool<Postgres>;

/// Standard Result type with our AppError
pub type Result<T> = std::result::Result<T, AppError>;
