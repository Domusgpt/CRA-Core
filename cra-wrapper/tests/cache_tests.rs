//! ContextCache tests

use cra_wrapper::cache::{ContextCache, CachedContext};
use cra_wrapper::config::{CacheConfig, CacheBackendType};
use chrono::{Duration, Utc};

fn test_cache_config() -> CacheConfig {
    CacheConfig {
        enabled: true,
        default_ttl_seconds: 3600,
        max_entries: 100,
        backend: CacheBackendType::Memory,
    }
}

#[tokio::test]
async fn test_cache_creation() {
    let config = CacheConfig::default();
    let cache = ContextCache::new(config);

    let stats = cache.stats().await;
    assert_eq!(stats.entry_count, 0);
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
}

#[tokio::test]
async fn test_cache_set_and_get() {
    let config = test_cache_config();
    let cache = ContextCache::new(config);

    let context = CachedContext {
        context_id: "ctx-1".to_string(),
        content: "Test content".to_string(),
        fetched_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(1),
        priority: 100,
    };

    cache.set("ctx-1", context.clone()).await;

    let retrieved = cache.get("ctx-1").await;
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.context_id, "ctx-1");
    assert_eq!(retrieved.content, "Test content");
    assert_eq!(retrieved.priority, 100);
}

#[tokio::test]
async fn test_cache_miss() {
    let config = test_cache_config();
    let cache = ContextCache::new(config);

    let retrieved = cache.get("non-existent").await;
    assert!(retrieved.is_none());

    let stats = cache.stats().await;
    assert_eq!(stats.misses, 1);
}

#[tokio::test]
async fn test_cache_hit_increments_counter() {
    let config = test_cache_config();
    let cache = ContextCache::new(config);

    let context = CachedContext {
        context_id: "ctx-1".to_string(),
        content: "Content".to_string(),
        fetched_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(1),
        priority: 100,
    };

    cache.set("ctx-1", context).await;

    // Multiple gets
    cache.get("ctx-1").await;
    cache.get("ctx-1").await;
    cache.get("ctx-1").await;

    let stats = cache.stats().await;
    assert_eq!(stats.hits, 3);
}

#[tokio::test]
async fn test_cache_expired_entry_returns_none() {
    let config = test_cache_config();
    let cache = ContextCache::new(config);

    // Create an already-expired context
    let context = CachedContext {
        context_id: "ctx-1".to_string(),
        content: "Content".to_string(),
        fetched_at: Utc::now() - Duration::hours(2),
        expires_at: Utc::now() - Duration::hours(1), // Already expired
        priority: 100,
    };

    cache.set("ctx-1", context).await;

    // Should return None for expired entry
    let retrieved = cache.get("ctx-1").await;
    assert!(retrieved.is_none());

    let stats = cache.stats().await;
    assert_eq!(stats.misses, 1);
}

#[tokio::test]
async fn test_cache_disabled() {
    let config = CacheConfig {
        enabled: false, // Cache disabled
        default_ttl_seconds: 3600,
        max_entries: 100,
        backend: CacheBackendType::Memory,
    };
    let cache = ContextCache::new(config);

    let context = CachedContext {
        context_id: "ctx-1".to_string(),
        content: "Content".to_string(),
        fetched_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(1),
        priority: 100,
    };

    cache.set("ctx-1", context).await;

    // Should return None when cache is disabled
    let retrieved = cache.get("ctx-1").await;
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_cache_invalidate() {
    let config = test_cache_config();
    let cache = ContextCache::new(config);

    let context = CachedContext {
        context_id: "ctx-1".to_string(),
        content: "Content".to_string(),
        fetched_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(1),
        priority: 100,
    };

    cache.set("ctx-1", context).await;

    // Verify it's there
    assert!(cache.get("ctx-1").await.is_some());

    // Invalidate
    cache.invalidate("ctx-1").await;

    // Should be gone
    assert!(cache.get("ctx-1").await.is_none());
}

#[tokio::test]
async fn test_cache_clear() {
    let config = test_cache_config();
    let cache = ContextCache::new(config);

    // Add multiple entries
    for i in 0..5 {
        let context = CachedContext {
            context_id: format!("ctx-{}", i),
            content: format!("Content {}", i),
            fetched_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(1),
            priority: 100,
        };
        cache.set(&format!("ctx-{}", i), context).await;
    }

    let stats = cache.stats().await;
    assert_eq!(stats.entry_count, 5);

    // Clear
    cache.clear().await;

    let stats = cache.stats().await;
    assert_eq!(stats.entry_count, 0);
}

#[tokio::test]
async fn test_cache_eviction_at_capacity() {
    let config = CacheConfig {
        enabled: true,
        default_ttl_seconds: 3600,
        max_entries: 3, // Small capacity for testing
        backend: CacheBackendType::Memory,
    };
    let cache = ContextCache::new(config);

    // Add entries up to capacity
    for i in 0..3 {
        let context = CachedContext {
            context_id: format!("ctx-{}", i),
            content: format!("Content {}", i),
            fetched_at: Utc::now() + Duration::seconds(i as i64), // Different times
            expires_at: Utc::now() + Duration::hours(1),
            priority: 100,
        };
        cache.set(&format!("ctx-{}", i), context).await;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // Add one more (should evict oldest)
    let new_context = CachedContext {
        context_id: "ctx-new".to_string(),
        content: "New content".to_string(),
        fetched_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(1),
        priority: 100,
    };
    cache.set("ctx-new", new_context).await;

    // Should still have only 3 entries
    let stats = cache.stats().await;
    assert_eq!(stats.entry_count, 3);

    // Should have evicted oldest (ctx-0)
    assert!(cache.get("ctx-0").await.is_none());

    // New entry should be present
    assert!(cache.get("ctx-new").await.is_some());

    // Eviction count should be incremented
    assert_eq!(stats.evictions, 1);
}

#[tokio::test]
async fn test_cache_evict_expired() {
    let config = test_cache_config();
    let cache = ContextCache::new(config);

    // Add a mix of expired and valid entries
    let expired = CachedContext {
        context_id: "expired".to_string(),
        content: "Expired".to_string(),
        fetched_at: Utc::now() - Duration::hours(2),
        expires_at: Utc::now() - Duration::hours(1),
        priority: 100,
    };

    let valid = CachedContext {
        context_id: "valid".to_string(),
        content: "Valid".to_string(),
        fetched_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(1),
        priority: 100,
    };

    cache.set("expired", expired).await;
    cache.set("valid", valid).await;

    // Evict expired
    cache.evict_expired().await;

    let stats = cache.stats().await;
    assert_eq!(stats.entry_count, 1);
    assert_eq!(stats.evictions, 1);

    // Valid entry should still be there
    assert!(cache.get("valid").await.is_some());
}

#[tokio::test]
async fn test_cache_hit_rate() {
    let config = test_cache_config();
    let cache = ContextCache::new(config);

    let context = CachedContext {
        context_id: "ctx-1".to_string(),
        content: "Content".to_string(),
        fetched_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(1),
        priority: 100,
    };

    cache.set("ctx-1", context).await;

    // 3 hits
    cache.get("ctx-1").await;
    cache.get("ctx-1").await;
    cache.get("ctx-1").await;

    // 1 miss
    cache.get("non-existent").await;

    let stats = cache.stats().await;
    assert_eq!(stats.hits, 3);
    assert_eq!(stats.misses, 1);
    assert!((stats.hit_rate - 0.75).abs() < 0.001); // 75% hit rate
}

#[tokio::test]
async fn test_cached_context_is_expired() {
    let expired = CachedContext {
        context_id: "ctx".to_string(),
        content: "".to_string(),
        fetched_at: Utc::now(),
        expires_at: Utc::now() - Duration::seconds(1),
        priority: 0,
    };

    let valid = CachedContext {
        context_id: "ctx".to_string(),
        content: "".to_string(),
        fetched_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(1),
        priority: 0,
    };

    assert!(expired.is_expired());
    assert!(!valid.is_expired());
}

#[tokio::test]
async fn test_cache_update_existing() {
    let config = test_cache_config();
    let cache = ContextCache::new(config);

    let context1 = CachedContext {
        context_id: "ctx-1".to_string(),
        content: "Original".to_string(),
        fetched_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(1),
        priority: 100,
    };

    let context2 = CachedContext {
        context_id: "ctx-1".to_string(),
        content: "Updated".to_string(),
        fetched_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(1),
        priority: 200,
    };

    cache.set("ctx-1", context1).await;
    cache.set("ctx-1", context2).await;

    let retrieved = cache.get("ctx-1").await.unwrap();
    assert_eq!(retrieved.content, "Updated");
    assert_eq!(retrieved.priority, 200);

    // Entry count should still be 1
    let stats = cache.stats().await;
    assert_eq!(stats.entry_count, 1);
}

#[tokio::test]
async fn test_cache_stats_serialization() {
    let config = test_cache_config();
    let cache = ContextCache::new(config);

    let stats = cache.stats().await;
    let json = serde_json::to_string(&stats).unwrap();

    assert!(json.contains("entry_count"));
    assert!(json.contains("hits"));
    assert!(json.contains("hit_rate"));
}
