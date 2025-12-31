//! Performance optimizations for CSPM.
//!
//! This module provides high-performance implementations:
//! - SIMD-accelerated Voronoi lookup
//! - Batch encoding/decoding
//! - Zero-copy buffer management
//! - Precomputed lookup tables

pub mod simd;
pub mod batch;

pub use simd::{SimdVoronoi, SimdQuaternion};
pub use batch::{BatchEncoder, BatchDecoder, BatchResult};

/// Performance configuration
#[derive(Clone, Debug)]
pub struct PerformanceConfig {
    /// Enable SIMD acceleration (if available)
    pub enable_simd: bool,
    /// Batch size for parallel processing
    pub batch_size: usize,
    /// Preallocate buffers
    pub preallocate_buffers: bool,
    /// Buffer pool size
    pub buffer_pool_size: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_simd: true,
            batch_size: 64,
            preallocate_buffers: true,
            buffer_pool_size: 16,
        }
    }
}

/// Performance metrics
#[derive(Clone, Debug, Default)]
pub struct PerformanceMetrics {
    /// Symbols processed
    pub symbols_processed: u64,
    /// Total processing time (nanoseconds)
    pub total_time_ns: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// SIMD operations used
    pub simd_operations: u64,
    /// Fallback operations used
    pub fallback_operations: u64,
}

impl PerformanceMetrics {
    /// Average time per symbol in nanoseconds
    pub fn avg_time_per_symbol_ns(&self) -> f64 {
        if self.symbols_processed == 0 {
            return 0.0;
        }
        self.total_time_ns as f64 / self.symbols_processed as f64
    }

    /// Cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            return 0.0;
        }
        self.cache_hits as f64 / total as f64
    }

    /// SIMD utilization rate
    pub fn simd_utilization(&self) -> f64 {
        let total = self.simd_operations + self.fallback_operations;
        if total == 0 {
            return 0.0;
        }
        self.simd_operations as f64 / total as f64
    }

    /// Reset metrics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_config_default() {
        let config = PerformanceConfig::default();
        assert!(config.enable_simd);
        assert_eq!(config.batch_size, 64);
    }

    #[test]
    fn test_metrics_calculations() {
        let metrics = PerformanceMetrics {
            symbols_processed: 1000,
            total_time_ns: 50000,
            cache_hits: 800,
            cache_misses: 200,
            simd_operations: 900,
            fallback_operations: 100,
        };

        assert_eq!(metrics.avg_time_per_symbol_ns(), 50.0);
        assert_eq!(metrics.cache_hit_rate(), 0.8);
        assert_eq!(metrics.simd_utilization(), 0.9);
    }
}
