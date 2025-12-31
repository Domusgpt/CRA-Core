//! Context block cache
//!
//! Caches context blocks to avoid redundant fetches/loads.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use super::DEFAULT_CONTEXT_TTL;

/// Configuration for context cache
#[derive(Debug, Clone)]
pub struct ContextCacheConfig {
    /// Default TTL for cached contexts
    pub default_ttl: Duration,
    /// Maximum number of entries
    pub max_entries: usize,
    /// Whether to track content hashes for verification
    pub track_hashes: bool,
}

impl Default for ContextCacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: DEFAULT_CONTEXT_TTL,
            max_entries: 1000,
            track_hashes: true,
        }
    }
}

impl ContextCacheConfig {
    /// Set default TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Set max entries
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }
}

/// A cached context block
#[derive(Debug, Clone)]
pub struct CachedContext {
    /// The context content
    pub content: String,
    /// When this entry was cached
    pub cached_at: Instant,
    /// When this entry expires
    pub expires_at: Instant,
    /// Content hash (for verification)
    pub content_hash: String,
    /// Atlas ID this context belongs to
    pub atlas_id: String,
    /// Context ID
    pub context_id: String,
}

impl CachedContext {
    /// Check if this entry has expired
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Get time until expiration
    pub fn ttl_remaining(&self) -> Duration {
        self.expires_at.saturating_duration_since(Instant::now())
    }
}

/// Cache key: atlas_id:context_id
fn make_key(atlas_id: &str, context_id: &str) -> String {
    format!("{}:{}", atlas_id, context_id)
}

/// Compute content hash
fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Context cache
#[derive(Debug)]
pub struct ContextCache {
    /// Cached entries
    entries: RwLock<HashMap<String, CachedContext>>,
    /// Configuration
    config: ContextCacheConfig,
    /// Statistics
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
}

impl ContextCache {
    /// Create a new cache with default config
    pub fn new() -> Self {
        Self::with_config(ContextCacheConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: ContextCacheConfig) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            config,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    /// Get a cached context
    pub fn get(&self, atlas_id: &str, context_id: &str) -> Option<CachedContext> {
        let key = make_key(atlas_id, context_id);
        let entries = self.entries.read().unwrap();

        match entries.get(&key) {
            Some(entry) if !entry.is_expired() => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(entry.clone())
            }
            Some(_) => {
                // Expired entry - count as miss
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Set a cached context
    pub fn set(
        &self,
        atlas_id: &str,
        context_id: &str,
        content: String,
        ttl: Option<Duration>,
    ) {
        let key = make_key(atlas_id, context_id);
        let now = Instant::now();
        let ttl = ttl.unwrap_or(self.config.default_ttl);

        let content_hash = if self.config.track_hashes {
            compute_hash(&content)
        } else {
            String::new()
        };

        let entry = CachedContext {
            content,
            cached_at: now,
            expires_at: now + ttl,
            content_hash,
            atlas_id: atlas_id.to_string(),
            context_id: context_id.to_string(),
        };

        let mut entries = self.entries.write().unwrap();

        // Check if we need to evict entries
        if entries.len() >= self.config.max_entries {
            self.evict_expired(&mut entries);

            // If still full, evict oldest
            if entries.len() >= self.config.max_entries {
                self.evict_oldest(&mut entries);
            }
        }

        entries.insert(key, entry);
    }

    /// Invalidate a specific context
    pub fn invalidate(&self, atlas_id: &str, context_id: &str) {
        let key = make_key(atlas_id, context_id);
        self.entries.write().unwrap().remove(&key);
    }

    /// Invalidate all contexts for an atlas
    pub fn invalidate_atlas(&self, atlas_id: &str) {
        let mut entries = self.entries.write().unwrap();
        let prefix = format!("{}:", atlas_id);
        entries.retain(|k, _| !k.starts_with(&prefix));
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.entries.write().unwrap().clear();
    }

    /// Evict expired entries
    fn evict_expired(&self, entries: &mut HashMap<String, CachedContext>) {
        let before = entries.len();
        entries.retain(|_, v| !v.is_expired());
        let evicted = before - entries.len();
        self.evictions.fetch_add(evicted as u64, Ordering::Relaxed);
    }

    /// Evict oldest entry
    fn evict_oldest(&self, entries: &mut HashMap<String, CachedContext>) {
        if let Some(oldest_key) = entries
            .iter()
            .min_by_key(|(_, v)| v.cached_at)
            .map(|(k, _)| k.clone())
        {
            entries.remove(&oldest_key);
            self.evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let entries = self.entries.read().unwrap();
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);

        CacheStats {
            entries: entries.len(),
            max_entries: self.config.max_entries,
            hits,
            misses,
            hit_rate: if hits + misses > 0 {
                hits as f64 / (hits + misses) as f64
            } else {
                0.0
            },
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.read().unwrap().is_empty()
    }
}

impl Default for ContextCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Current number of entries
    pub entries: usize,
    /// Maximum entries allowed
    pub max_entries: usize,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Hit rate (0.0 - 1.0)
    pub hit_rate: f64,
    /// Total evictions
    pub evictions: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_basic_cache() {
        let cache = ContextCache::new();

        // Miss on empty cache
        assert!(cache.get("atlas-1", "context-1").is_none());

        // Add entry
        cache.set("atlas-1", "context-1", "Test content".to_string(), None);

        // Hit
        let entry = cache.get("atlas-1", "context-1");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().content, "Test content");

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_ttl_expiration() {
        let config = ContextCacheConfig::default()
            .with_ttl(Duration::from_millis(50));
        let cache = ContextCache::with_config(config);

        cache.set("atlas-1", "context-1", "Test content".to_string(), None);

        // Should be cached
        assert!(cache.get("atlas-1", "context-1").is_some());

        // Wait for expiration
        thread::sleep(Duration::from_millis(60));

        // Should be expired
        assert!(cache.get("atlas-1", "context-1").is_none());
    }

    #[test]
    fn test_invalidation() {
        let cache = ContextCache::new();

        cache.set("atlas-1", "context-1", "Content 1".to_string(), None);
        cache.set("atlas-1", "context-2", "Content 2".to_string(), None);
        cache.set("atlas-2", "context-1", "Content 3".to_string(), None);

        assert_eq!(cache.len(), 3);

        // Invalidate single context
        cache.invalidate("atlas-1", "context-1");
        assert_eq!(cache.len(), 2);
        assert!(cache.get("atlas-1", "context-1").is_none());

        // Invalidate entire atlas
        cache.invalidate_atlas("atlas-1");
        assert_eq!(cache.len(), 1);
        assert!(cache.get("atlas-2", "context-1").is_some());
    }

    #[test]
    fn test_content_hash() {
        let cache = ContextCache::new();

        cache.set("atlas-1", "context-1", "Test content".to_string(), None);

        let entry = cache.get("atlas-1", "context-1").unwrap();
        assert!(!entry.content_hash.is_empty());

        // Same content should produce same hash
        let hash1 = compute_hash("Test content");
        assert_eq!(entry.content_hash, hash1);
    }

    #[test]
    fn test_eviction_on_max_entries() {
        let config = ContextCacheConfig::default()
            .with_max_entries(3);
        let cache = ContextCache::with_config(config);

        cache.set("atlas-1", "context-1", "Content 1".to_string(), None);
        cache.set("atlas-1", "context-2", "Content 2".to_string(), None);
        cache.set("atlas-1", "context-3", "Content 3".to_string(), None);

        assert_eq!(cache.len(), 3);

        // Adding 4th should trigger eviction
        cache.set("atlas-1", "context-4", "Content 4".to_string(), None);

        assert_eq!(cache.len(), 3);
        assert!(cache.stats().evictions > 0);
    }
}
