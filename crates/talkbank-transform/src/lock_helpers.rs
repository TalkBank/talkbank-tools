//! Lock acquisition helpers with poison recovery
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>

use std::sync::{Mutex, MutexGuard};

/// Error type for lock operations
#[derive(Debug)]
#[allow(dead_code)]
pub enum LockError {
    Poisoned(String),
}

impl std::fmt::Display for LockError {
    /// Render lock acquisition failures.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::Poisoned(msg) => write!(f, "Lock poisoned: {}", msg),
        }
    }
}

impl std::error::Error for LockError {}

/// Acquire a mutex lock, recovering from poisoning
///
/// When a thread panics while holding a lock, the lock is "poisoned".
/// This function recovers the lock by extracting the inner value,
/// allowing the application to continue.
///
/// **Warning**: Recovering from a poisoned lock means the protected
/// data may be in an inconsistent state. Use with caution.
#[allow(dead_code)]
pub fn acquire_lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, LockError> {
    match mutex.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            tracing::warn!(
                "Mutex lock was poisoned (a thread panicked while holding it). \
                 Recovering, but data may be inconsistent."
            );
            Ok(poisoned.into_inner())
        }
    }
}
