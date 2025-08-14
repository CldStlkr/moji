use crate::{error::AppError, LobbyState};
use sqlx::{Pool, Postgres};
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

// Newtype wrapper for Arc<Mutex<T>>
#[derive(Clone)]
pub struct Shared<T>(Arc<Mutex<T>>);

impl<T> Shared<T> {
    /// Create a new shared value
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }

    /// Execute a closure with mutable access to the shared value
    /// Automatically converts poison errors to AppError
    pub fn with<F, R>(&self, f: F) -> crate::types::Result<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self
            .0
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        Ok(f(&mut *guard))
    }

    /// Execute a closure with read-only access to the shared value
    /// Automatically converts poison errors to AppError
    pub fn read<F, R>(&self, f: F) -> crate::types::Result<R>
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self
            .0
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        Ok(f(&*guard))
    }

    ///// Get a mutable reference to the inner value, with automatic error conversion
    //pub fn lock_safe(&self) -> crate::types::Result<std::sync::MutexGuard<T>> {
    //    self.0
    //        .lock()
    //        .map_err(|e| AppError::LockError(e.to_string()))
    //}
}

// Deref implementation for drop-in compatibility with Arc<Mutex<T>>
impl<T> Deref for Shared<T> {
    type Target = Arc<Mutex<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Shared lobby state reference
pub type SharedState = Arc<LobbyState>;

/// PostgreSQL connection pool
pub type DbPool = Pool<Postgres>;

/// Standard Result type with our AppError
pub type Result<T> = std::result::Result<T, AppError>;
