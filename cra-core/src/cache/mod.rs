//! Caching layer for CRA
//!
//! Provides caching for:
//! - Context blocks (avoid re-fetching)
//! - Policy decisions (avoid re-evaluating)
//!
//! Features:
//! - TTL (time-to-live) support
//! - Cache invalidation
//! - Statistics tracking

mod context_cache;
mod policy_cache;

pub use context_cache::{ContextCache, CachedContext, ContextCacheConfig};
pub use policy_cache::{PolicyCache, CachedPolicy, PolicyCacheConfig};

use std::time::Duration;

/// Default TTL for context blocks (5 minutes)
pub const DEFAULT_CONTEXT_TTL: Duration = Duration::from_secs(300);

/// Default TTL for policy decisions (1 minute)
pub const DEFAULT_POLICY_TTL: Duration = Duration::from_secs(60);

/// Combined cache for CRA
#[derive(Debug)]
pub struct CRACache {
    /// Context block cache
    pub contexts: ContextCache,
    /// Policy decision cache
    pub policies: PolicyCache,
}

impl CRACache {
    /// Create a new cache with default configuration
    pub fn new() -> Self {
        Self {
            contexts: ContextCache::new(),
            policies: PolicyCache::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(context_config: ContextCacheConfig, policy_config: PolicyCacheConfig) -> Self {
        Self {
            contexts: ContextCache::with_config(context_config),
            policies: PolicyCache::with_config(policy_config),
        }
    }

    /// Clear all caches
    pub fn clear(&self) {
        self.contexts.clear();
        self.policies.clear();
    }

    /// Invalidate a specific atlas (clears related context and policy entries)
    pub fn invalidate_atlas(&self, atlas_id: &str) {
        self.contexts.invalidate_atlas(atlas_id);
        self.policies.invalidate_atlas(atlas_id);
    }

    /// Get combined cache statistics
    pub fn stats(&self) -> CacheCombinedStats {
        CacheCombinedStats {
            context_stats: self.contexts.stats(),
            policy_stats: self.policies.stats(),
        }
    }
}

impl Default for CRACache {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined cache statistics
#[derive(Debug, Clone)]
pub struct CacheCombinedStats {
    pub context_stats: context_cache::CacheStats,
    pub policy_stats: policy_cache::CacheStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combined_cache() {
        let cache = CRACache::new();

        // Add some context
        cache.contexts.set(
            "atlas-1",
            "context-1",
            "Test content".to_string(),
            None,
        );

        // Add some policy
        cache.policies.set(
            "atlas-1",
            "action-1",
            "params-hash",
            policy_cache::PolicyDecision::Allow,
            None,
        );

        // Verify both are cached
        assert!(cache.contexts.get("atlas-1", "context-1").is_some());
        assert!(cache.policies.get("atlas-1", "action-1", "params-hash").is_some());

        // Invalidate atlas
        cache.invalidate_atlas("atlas-1");

        // Both should be cleared
        assert!(cache.contexts.get("atlas-1", "context-1").is_none());
        assert!(cache.policies.get("atlas-1", "action-1", "params-hash").is_none());
    }
}
