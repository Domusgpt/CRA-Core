//! Policy decision cache
//!
//! Caches policy evaluation results to avoid re-evaluating.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use super::DEFAULT_POLICY_TTL;

/// Configuration for policy cache
#[derive(Debug, Clone)]
pub struct PolicyCacheConfig {
    /// Default TTL for cached decisions
    pub default_ttl: Duration,
    /// Maximum number of entries
    pub max_entries: usize,
}

impl Default for PolicyCacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: DEFAULT_POLICY_TTL,
            max_entries: 500,
        }
    }
}

impl PolicyCacheConfig {
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

/// Policy decision types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Action is allowed
    Allow,
    /// Action is denied
    Deny,
    /// Action requires approval
    RequireApproval,
    /// Allow with constraints
    AllowWithConstraints,
}

impl PolicyDecision {
    /// Check if this decision allows the action
    pub fn is_allowed(&self) -> bool {
        matches!(self, PolicyDecision::Allow | PolicyDecision::AllowWithConstraints)
    }

    /// Check if this decision denies the action
    pub fn is_denied(&self) -> bool {
        matches!(self, PolicyDecision::Deny)
    }
}

/// A cached policy decision
#[derive(Debug, Clone)]
pub struct CachedPolicy {
    /// The decision
    pub decision: PolicyDecision,
    /// Optional reason for the decision
    pub reason: Option<String>,
    /// Policy ID that made this decision
    pub policy_id: Option<String>,
    /// When this entry was cached
    pub cached_at: Instant,
    /// When this entry expires
    pub expires_at: Instant,
    /// Atlas ID
    pub atlas_id: String,
    /// Action ID
    pub action_id: String,
    /// Parameters hash (for matching)
    pub params_hash: String,
}

impl CachedPolicy {
    /// Check if this entry has expired
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Get time until expiration
    pub fn ttl_remaining(&self) -> Duration {
        self.expires_at.saturating_duration_since(Instant::now())
    }
}

/// Cache key: atlas_id:action_id:params_hash
fn make_key(atlas_id: &str, action_id: &str, params_hash: &str) -> String {
    format!("{}:{}:{}", atlas_id, action_id, params_hash)
}

/// Compute hash of parameters for cache key
pub fn hash_params(params: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();
    // Use canonical JSON for consistent hashing
    let json = serde_json::to_string(params).unwrap_or_default();
    hasher.update(json.as_bytes());
    hex::encode(hasher.finalize())
}

/// Policy cache
#[derive(Debug)]
pub struct PolicyCache {
    /// Cached entries
    entries: RwLock<HashMap<String, CachedPolicy>>,
    /// Configuration
    config: PolicyCacheConfig,
    /// Statistics
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
}

impl PolicyCache {
    /// Create a new cache with default config
    pub fn new() -> Self {
        Self::with_config(PolicyCacheConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: PolicyCacheConfig) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            config,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    /// Get a cached policy decision
    pub fn get(&self, atlas_id: &str, action_id: &str, params_hash: &str) -> Option<CachedPolicy> {
        let key = make_key(atlas_id, action_id, params_hash);
        let entries = self.entries.read().unwrap();

        match entries.get(&key) {
            Some(entry) if !entry.is_expired() => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(entry.clone())
            }
            Some(_) => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Get by action params (computes hash internally)
    pub fn get_by_params(
        &self,
        atlas_id: &str,
        action_id: &str,
        params: &serde_json::Value,
    ) -> Option<CachedPolicy> {
        let params_hash = hash_params(params);
        self.get(atlas_id, action_id, &params_hash)
    }

    /// Set a cached policy decision
    pub fn set(
        &self,
        atlas_id: &str,
        action_id: &str,
        params_hash: &str,
        decision: PolicyDecision,
        ttl: Option<Duration>,
    ) {
        self.set_full(atlas_id, action_id, params_hash, decision, None, None, ttl);
    }

    /// Set a cached policy decision with full details
    pub fn set_full(
        &self,
        atlas_id: &str,
        action_id: &str,
        params_hash: &str,
        decision: PolicyDecision,
        reason: Option<String>,
        policy_id: Option<String>,
        ttl: Option<Duration>,
    ) {
        let key = make_key(atlas_id, action_id, params_hash);
        let now = Instant::now();
        let ttl = ttl.unwrap_or(self.config.default_ttl);

        let entry = CachedPolicy {
            decision,
            reason,
            policy_id,
            cached_at: now,
            expires_at: now + ttl,
            atlas_id: atlas_id.to_string(),
            action_id: action_id.to_string(),
            params_hash: params_hash.to_string(),
        };

        let mut entries = self.entries.write().unwrap();

        // Evict if needed
        if entries.len() >= self.config.max_entries {
            self.evict_expired(&mut entries);

            if entries.len() >= self.config.max_entries {
                self.evict_oldest(&mut entries);
            }
        }

        entries.insert(key, entry);
    }

    /// Invalidate a specific action
    pub fn invalidate(&self, atlas_id: &str, action_id: &str, params_hash: &str) {
        let key = make_key(atlas_id, action_id, params_hash);
        self.entries.write().unwrap().remove(&key);
    }

    /// Invalidate all decisions for an action (any params)
    pub fn invalidate_action(&self, atlas_id: &str, action_id: &str) {
        let mut entries = self.entries.write().unwrap();
        let prefix = format!("{}:{}:", atlas_id, action_id);
        entries.retain(|k, _| !k.starts_with(&prefix));
    }

    /// Invalidate all decisions for an atlas
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
    fn evict_expired(&self, entries: &mut HashMap<String, CachedPolicy>) {
        let before = entries.len();
        entries.retain(|_, v| !v.is_expired());
        let evicted = before - entries.len();
        self.evictions.fetch_add(evicted as u64, Ordering::Relaxed);
    }

    /// Evict oldest entry
    fn evict_oldest(&self, entries: &mut HashMap<String, CachedPolicy>) {
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

impl Default for PolicyCache {
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
    use serde_json::json;
    use std::thread;

    #[test]
    fn test_basic_cache() {
        let cache = PolicyCache::new();

        // Miss on empty cache
        assert!(cache.get("atlas-1", "action-1", "hash-1").is_none());

        // Add entry
        cache.set("atlas-1", "action-1", "hash-1", PolicyDecision::Allow, None);

        // Hit
        let entry = cache.get("atlas-1", "action-1", "hash-1");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().decision, PolicyDecision::Allow);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_decision_types() {
        let cache = PolicyCache::new();

        cache.set("atlas-1", "action-allow", "h", PolicyDecision::Allow, None);
        cache.set("atlas-1", "action-deny", "h", PolicyDecision::Deny, None);
        cache.set("atlas-1", "action-approve", "h", PolicyDecision::RequireApproval, None);

        assert!(cache.get("atlas-1", "action-allow", "h").unwrap().decision.is_allowed());
        assert!(cache.get("atlas-1", "action-deny", "h").unwrap().decision.is_denied());
        assert!(!cache.get("atlas-1", "action-approve", "h").unwrap().decision.is_allowed());
    }

    #[test]
    fn test_ttl_expiration() {
        let config = PolicyCacheConfig::default()
            .with_ttl(Duration::from_millis(50));
        let cache = PolicyCache::with_config(config);

        cache.set("atlas-1", "action-1", "hash-1", PolicyDecision::Allow, None);

        // Should be cached
        assert!(cache.get("atlas-1", "action-1", "hash-1").is_some());

        // Wait for expiration
        thread::sleep(Duration::from_millis(60));

        // Should be expired
        assert!(cache.get("atlas-1", "action-1", "hash-1").is_none());
    }

    #[test]
    fn test_get_by_params() {
        let cache = PolicyCache::new();
        let params = json!({"key": "value", "num": 42});
        let params_hash = hash_params(&params);

        cache.set("atlas-1", "action-1", &params_hash, PolicyDecision::Allow, None);

        // Should find by params
        let entry = cache.get_by_params("atlas-1", "action-1", &params);
        assert!(entry.is_some());

        // Different params should miss
        let other_params = json!({"key": "other"});
        assert!(cache.get_by_params("atlas-1", "action-1", &other_params).is_none());
    }

    #[test]
    fn test_invalidation() {
        let cache = PolicyCache::new();

        cache.set("atlas-1", "action-1", "hash-1", PolicyDecision::Allow, None);
        cache.set("atlas-1", "action-1", "hash-2", PolicyDecision::Deny, None);
        cache.set("atlas-1", "action-2", "hash-1", PolicyDecision::Allow, None);
        cache.set("atlas-2", "action-1", "hash-1", PolicyDecision::Allow, None);

        assert_eq!(cache.len(), 4);

        // Invalidate single entry
        cache.invalidate("atlas-1", "action-1", "hash-1");
        assert_eq!(cache.len(), 3);

        // Invalidate action (any params)
        cache.invalidate_action("atlas-1", "action-1");
        assert_eq!(cache.len(), 2);

        // Invalidate entire atlas
        cache.invalidate_atlas("atlas-1");
        assert_eq!(cache.len(), 1);
        assert!(cache.get("atlas-2", "action-1", "hash-1").is_some());
    }

    #[test]
    fn test_full_details() {
        let cache = PolicyCache::new();

        cache.set_full(
            "atlas-1",
            "action-1",
            "hash-1",
            PolicyDecision::Deny,
            Some("Policy violation".to_string()),
            Some("policy-123".to_string()),
            None,
        );

        let entry = cache.get("atlas-1", "action-1", "hash-1").unwrap();
        assert_eq!(entry.decision, PolicyDecision::Deny);
        assert_eq!(entry.reason, Some("Policy violation".to_string()));
        assert_eq!(entry.policy_id, Some("policy-123".to_string()));
    }
}
