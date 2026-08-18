//! Shared application state.

use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Live WebSocket connections keyed by user id. Each user has a single
/// connection in the MVP (newer connections replace older ones).
#[derive(Clone, Default)]
pub struct Connections {
    pub map: Arc<Mutex<HashMap<i64, mpsc::UnboundedSender<String>>>>,
}

impl Connections {
    /// Register a connection for a user, returning the receiver to drain.
    pub fn register(&self, user_id: i64) -> mpsc::UnboundedReceiver<String> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.map.lock().unwrap().insert(user_id, tx);
        rx
    }

    /// Remove a user's connection (on disconnect).
    pub fn unregister(&self, user_id: i64) {
        self.map.lock().unwrap().remove(&user_id);
    }

    /// Push a message to a user's connection if online.
    pub fn send(&self, user_id: i64, message: String) {
        if let Some(tx) = self.map.lock().unwrap().get(&user_id) {
            let _ = tx.send(message);
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    /// HMAC secret for signing JWTs.
    pub jwt_secret: String,
    pub connections: Connections,
}
