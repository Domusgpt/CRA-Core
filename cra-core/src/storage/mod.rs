//! Pluggable Storage Backend
//!
//! This module provides traits and implementations for persisting CRA data.
//! The default is in-memory storage, but users can implement custom backends
//! for SQLite, PostgreSQL, files, etc.
//!
//! # Example
//!
//! ```rust
//! use cra_core::storage::{StorageBackend, InMemoryStorage};
//!
//! // Default in-memory storage
//! let storage = InMemoryStorage::new();
//!
//! // Or use a custom backend
//! // let storage = SqliteStorage::new("traces.db")?;
//! ```

use std::collections::HashMap;
use std::sync::RwLock;

use crate::error::{CRAError, Result};
use crate::trace::TRACEEvent;

/// Storage backend trait for persisting traces
///
/// Implement this trait to add custom persistence backends.
/// All methods take `&self` to allow for interior mutability patterns.
pub trait StorageBackend: Send + Sync {
    /// Store a trace event
    fn store_event(&self, event: &TRACEEvent) -> Result<()>;

    /// Get all events for a session
    fn get_events(&self, session_id: &str) -> Result<Vec<TRACEEvent>>;

    /// Get events by type for a session
    fn get_events_by_type(&self, session_id: &str, event_type: &str) -> Result<Vec<TRACEEvent>>;

    /// Get the last N events for a session
    fn get_last_events(&self, session_id: &str, n: usize) -> Result<Vec<TRACEEvent>>;

    /// Get event count for a session
    fn get_event_count(&self, session_id: &str) -> Result<usize>;

    /// Delete all events for a session
    fn delete_session(&self, session_id: &str) -> Result<()>;

    /// Check if backend is healthy
    fn health_check(&self) -> Result<()>;

    /// Get backend name (for logging/debugging)
    fn name(&self) -> &'static str;
}

/// In-memory storage backend (default)
///
/// Stores events in memory using a HashMap. Events are lost on restart.
/// Thread-safe via RwLock.
#[derive(Debug, Default)]
pub struct InMemoryStorage {
    events: RwLock<HashMap<String, Vec<TRACEEvent>>>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        Self {
            events: RwLock::new(HashMap::new()),
        }
    }

    /// Get total event count across all sessions
    pub fn total_events(&self) -> usize {
        self.events
            .read()
            .map(|e| e.values().map(|v| v.len()).sum())
            .unwrap_or(0)
    }

    /// Get all session IDs
    pub fn session_ids(&self) -> Vec<String> {
        self.events
            .read()
            .map(|e| e.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Clear all stored events
    pub fn clear(&self) {
        if let Ok(mut events) = self.events.write() {
            events.clear();
        }
    }
}

impl StorageBackend for InMemoryStorage {
    fn store_event(&self, event: &TRACEEvent) -> Result<()> {
        let mut events = self.events.write().map_err(|_| CRAError::StorageLocked)?;
        events
            .entry(event.session_id.clone())
            .or_default()
            .push(event.clone());
        Ok(())
    }

    fn get_events(&self, session_id: &str) -> Result<Vec<TRACEEvent>> {
        let events = self.events.read().map_err(|_| CRAError::StorageLocked)?;
        Ok(events.get(session_id).cloned().unwrap_or_default())
    }

    fn get_events_by_type(&self, session_id: &str, event_type: &str) -> Result<Vec<TRACEEvent>> {
        let events = self.events.read().map_err(|_| CRAError::StorageLocked)?;
        Ok(events
            .get(session_id)
            .map(|v| {
                v.iter()
                    .filter(|e| e.event_type.to_string() == event_type)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }

    fn get_last_events(&self, session_id: &str, n: usize) -> Result<Vec<TRACEEvent>> {
        let events = self.events.read().map_err(|_| CRAError::StorageLocked)?;
        Ok(events
            .get(session_id)
            .map(|v| v.iter().rev().take(n).rev().cloned().collect())
            .unwrap_or_default())
    }

    fn get_event_count(&self, session_id: &str) -> Result<usize> {
        let events = self.events.read().map_err(|_| CRAError::StorageLocked)?;
        Ok(events.get(session_id).map(|v| v.len()).unwrap_or(0))
    }

    fn delete_session(&self, session_id: &str) -> Result<()> {
        let mut events = self.events.write().map_err(|_| CRAError::StorageLocked)?;
        events.remove(session_id);
        Ok(())
    }

    fn health_check(&self) -> Result<()> {
        // In-memory is always healthy if we can acquire the lock
        let _events = self.events.read().map_err(|_| CRAError::StorageLocked)?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "in-memory"
    }
}

/// File-based storage backend (JSONL files)
///
/// Stores events as newline-delimited JSON files, one per session.
/// Suitable for development and small-scale deployments.
#[derive(Debug)]
pub struct FileStorage {
    directory: std::path::PathBuf,
}

impl FileStorage {
    /// Create a new file storage in the given directory
    pub fn new<P: Into<std::path::PathBuf>>(directory: P) -> Result<Self> {
        let dir = directory.into();
        std::fs::create_dir_all(&dir).map_err(|e| CRAError::IoError {
            message: format!("Failed to create storage directory: {}", e),
        })?;
        Ok(Self { directory: dir })
    }

    fn session_file(&self, session_id: &str) -> std::path::PathBuf {
        self.directory.join(format!("{}.jsonl", session_id))
    }
}

impl StorageBackend for FileStorage {
    fn store_event(&self, event: &TRACEEvent) -> Result<()> {
        use std::io::Write;

        let path = self.session_file(&event.session_id);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| CRAError::IoError {
                message: format!("Failed to open file: {}", e),
            })?;

        let line = serde_json::to_string(event)?;

        writeln!(file, "{}", line).map_err(|e| CRAError::IoError {
            message: format!("Failed to write: {}", e),
        })?;

        Ok(())
    }

    fn get_events(&self, session_id: &str) -> Result<Vec<TRACEEvent>> {
        use std::io::BufRead;

        let path = self.session_file(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = std::fs::File::open(&path).map_err(|e| CRAError::IoError {
            message: format!("Failed to open file: {}", e),
        })?;

        let reader = std::io::BufReader::new(file);
        let mut events = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| CRAError::IoError {
                message: format!("Failed to read line: {}", e),
            })?;
            if !line.trim().is_empty() {
                let event: TRACEEvent = serde_json::from_str(&line)?;
                events.push(event);
            }
        }

        Ok(events)
    }

    fn get_events_by_type(&self, session_id: &str, event_type: &str) -> Result<Vec<TRACEEvent>> {
        let events = self.get_events(session_id)?;
        Ok(events
            .into_iter()
            .filter(|e| e.event_type.to_string() == event_type)
            .collect())
    }

    fn get_last_events(&self, session_id: &str, n: usize) -> Result<Vec<TRACEEvent>> {
        let events = self.get_events(session_id)?;
        Ok(events.into_iter().rev().take(n).rev().collect())
    }

    fn get_event_count(&self, session_id: &str) -> Result<usize> {
        Ok(self.get_events(session_id)?.len())
    }

    fn delete_session(&self, session_id: &str) -> Result<()> {
        let path = self.session_file(session_id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| CRAError::IoError {
                message: format!("Failed to delete file: {}", e),
            })?;
        }
        Ok(())
    }

    fn health_check(&self) -> Result<()> {
        if self.directory.exists() && self.directory.is_dir() {
            Ok(())
        } else {
            Err(CRAError::IoError {
                message: "Storage directory does not exist".to_string(),
            })
        }
    }

    fn name(&self) -> &'static str {
        "file"
    }
}

/// Null storage backend (discards all events)
///
/// Useful for testing or when traces are not needed.
#[derive(Debug, Default, Clone)]
pub struct NullStorage;

impl NullStorage {
    pub fn new() -> Self {
        Self
    }
}

impl StorageBackend for NullStorage {
    fn store_event(&self, _event: &TRACEEvent) -> Result<()> {
        Ok(())
    }

    fn get_events(&self, _session_id: &str) -> Result<Vec<TRACEEvent>> {
        Ok(Vec::new())
    }

    fn get_events_by_type(&self, _session_id: &str, _event_type: &str) -> Result<Vec<TRACEEvent>> {
        Ok(Vec::new())
    }

    fn get_last_events(&self, _session_id: &str, _n: usize) -> Result<Vec<TRACEEvent>> {
        Ok(Vec::new())
    }

    fn get_event_count(&self, _session_id: &str) -> Result<usize> {
        Ok(0)
    }

    fn delete_session(&self, _session_id: &str) -> Result<()> {
        Ok(())
    }

    fn health_check(&self) -> Result<()> {
        Ok(())
    }

    fn name(&self) -> &'static str {
        "null"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::EventType;
    use serde_json::json;

    fn create_test_event(session_id: &str, seq: u64) -> TRACEEvent {
        TRACEEvent::new(
            session_id.to_string(),
            "trace-1".to_string(),
            EventType::SessionStarted,
            json!({"test": true}),
        )
        .chain(seq, "0".repeat(64))
    }

    #[test]
    fn test_in_memory_storage() {
        let storage = InMemoryStorage::new();

        let event = create_test_event("session-1", 0);
        storage.store_event(&event).unwrap();

        let events = storage.get_events("session-1").unwrap();
        assert_eq!(events.len(), 1);

        let count = storage.get_event_count("session-1").unwrap();
        assert_eq!(count, 1);

        storage.delete_session("session-1").unwrap();
        let events = storage.get_events("session-1").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_file_storage() {
        let temp_dir = std::env::temp_dir().join("cra-test-storage");
        let storage = FileStorage::new(&temp_dir).unwrap();

        let event = create_test_event("test-session", 0);
        storage.store_event(&event).unwrap();

        let events = storage.get_events("test-session").unwrap();
        assert_eq!(events.len(), 1);

        storage.delete_session("test-session").unwrap();

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_null_storage() {
        let storage = NullStorage::new();

        let event = create_test_event("session-1", 0);
        storage.store_event(&event).unwrap();

        // Null storage discards everything
        let events = storage.get_events("session-1").unwrap();
        assert!(events.is_empty());
    }
}
