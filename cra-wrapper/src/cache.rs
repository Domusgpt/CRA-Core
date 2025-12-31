//! Context cache for avoiding redundant fetches

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::CacheConfig;

/// A cached context block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedContext {
    /// Context identifier
    pub context_id: String,

    /// The content
    pub content: String,

    /// When it was fetched
    pub fetched_at: DateTime<Utc>,

    /// When it expires
    pub expires_at: DateTime<Utc>,

    /// Priority
    pub priority: i32,
}

impl CachedContext {
    /// Check if the context is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Number of entries in cache
    pub entry_count: usize,

    /// Total cache hits
    pub hits: u64,

    /// Total cache misses
    pub misses: u64,

    /// Hit rate (0.0 - 1.0)
    pub hit_rate: f64,

    /// Number of evictions
    pub evictions: u64,
}

/// Context cache
pub struct ContextCache {
    /// Cache configuration
    config: CacheConfig,

    /// Cached contexts by ID
    entries: RwLock<HashMap<String, CachedContext>>,

    /// Statistics
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
}

impl ContextCache {
    /// Create a new context cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            entries: RwLock::new(HashMap::new()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    /// Get a context from cache
    pub async fn get(&self, key: &str) -> Option<CachedContext> {
        if !self.config.enabled {
            self.misses.fetch_add(1, Ordering::SeqCst);
            return None;
        }

        let entries = self.entries.read().await;

        if let Some(ctx) = entries.get(key) {
            if !ctx.is_expired() {
                self.hits.fetch_add(1, Ordering::SeqCst);
                return Some(ctx.clone());
            }
        }

        self.misses.fetch_add(1, Ordering::SeqCst);
        None
    }

    /// Store a context in cache
    pub async fn set(&self, key: &str, context: CachedContext) {
        if !self.config.enabled {
            return;
        }

        let mut entries = self.entries.write().await;

        // Evict if at capacity
        if entries.len() >= self.config.max_entries && !entries.contains_key(key) {
            // Find and remove oldest entry
            if let Some(oldest_key) = entries.iter()
                .min_by_key(|(_, v)| v.fetched_at)
                .map(|(k, _)| k.clone())
            {
                entries.remove(&oldest_key);
                self.evictions.fetch_add(1, Ordering::SeqCst);
            }
        }

        entries.insert(key.to_string(), context);
    }

    /// Invalidate a cache entry
    pub async fn invalidate(&self, key: &str) {
        let mut entries = self.entries.write().await;
        entries.remove(key);
    }

    /// Clear the entire cache
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let entry_count = self.entries.read().await.len();
        let hits = self.hits.load(Ordering::SeqCst);
        let misses = self.misses.load(Ordering::SeqCst);
        let total = hits + misses;

        CacheStats {
            entry_count,
            hits,
            misses,
            hit_rate: if total > 0 { hits as f64 / total as f64 } else { 0.0 },
            evictions: self.evictions.load(Ordering::SeqCst),
        }
    }

    /// Remove expired entries
    pub async fn evict_expired(&self) {
        let mut entries = self.entries.write().await;
        let now = Utc::now();

        let expired: Vec<String> = entries.iter()
            .filter(|(_, v)| v.expires_at < now)
            .map(|(k, _)| k.clone())
            .collect();

        for key in expired {
            entries.remove(&key);
            self.evictions.fetch_add(1, Ordering::SeqCst);
        }
    }
}
