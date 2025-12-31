//! Batch encoding and decoding for high throughput.
//!
//! Provides efficient batch processing of CSPM symbols:
//! - Encode multiple bytes simultaneously
//! - Decode batches of received quaternions
//! - Preallocated buffer pools for zero-copy operation

use crate::quaternion::Quaternion;
use crate::polytope::{Hexacosichoron, GrayCodeMapper};
use crate::crypto::{HashChain, ChainState};
use super::simd::SimdVoronoi;
use super::PerformanceMetrics;

/// Configuration for batch processing
#[derive(Clone, Debug)]
pub struct BatchConfig {
    /// Symbols per batch
    pub batch_size: usize,
    /// Enable hash chain updates
    pub update_chain: bool,
    /// Enable automatic normalization
    pub auto_normalize: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 64,
            update_chain: true,
            auto_normalize: true,
        }
    }
}

/// Result of batch operation
#[derive(Clone, Debug)]
pub struct BatchResult {
    /// Number of items processed
    pub count: usize,
    /// Processing time in nanoseconds
    pub time_ns: u64,
    /// Errors encountered
    pub errors: usize,
}

impl BatchResult {
    /// Throughput in symbols per second
    pub fn throughput(&self) -> f64 {
        if self.time_ns == 0 {
            return 0.0;
        }
        (self.count as f64) / (self.time_ns as f64 / 1_000_000_000.0)
    }
}

/// High-performance batch encoder
pub struct BatchEncoder {
    config: BatchConfig,
    hexacosichoron: Hexacosichoron,
    gray_mapper: GrayCodeMapper,
    /// Precomputed vertex quaternions
    vertex_cache: Vec<Quaternion>,
    /// Current lattice rotation
    rotation: Quaternion,
    /// Metrics
    metrics: PerformanceMetrics,
}

impl BatchEncoder {
    /// Create new batch encoder
    pub fn new(config: BatchConfig) -> Self {
        let hexacosichoron = Hexacosichoron::new();
        let gray_mapper = GrayCodeMapper::new(&hexacosichoron);

        // Cache vertex quaternions for fast access
        let vertex_cache: Vec<Quaternion> = hexacosichoron
            .vertices()
            .iter()
            .map(|v| v.q)
            .collect();

        Self {
            config,
            hexacosichoron,
            gray_mapper,
            vertex_cache,
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            metrics: PerformanceMetrics::default(),
        }
    }

    /// Set lattice rotation from chain state
    pub fn set_rotation(&mut self, rotation: Quaternion) {
        self.rotation = rotation;
    }

    /// Encode a single symbol (7 bits)
    #[inline]
    pub fn encode_symbol(&self, symbol: u8) -> Quaternion {
        let vertex_idx = self.gray_mapper.encode(symbol & 0x7F).unwrap_or(0);
        let base_q = self.vertex_cache[vertex_idx];

        // Apply lattice rotation
        let rotated = self.rotation * base_q * self.rotation.conjugate();
        rotated.normalize()
    }

    /// Encode batch of symbols
    pub fn encode_batch(&mut self, symbols: &[u8]) -> Vec<Quaternion> {
        let start = std::time::Instant::now();

        let result: Vec<Quaternion> = symbols
            .iter()
            .map(|&s| self.encode_symbol(s))
            .collect();

        let elapsed = start.elapsed().as_nanos() as u64;
        self.metrics.symbols_processed += symbols.len() as u64;
        self.metrics.total_time_ns += elapsed;

        result
    }

    /// Encode batch of symbols to preallocated buffer
    pub fn encode_batch_into(&mut self, symbols: &[u8], output: &mut [Quaternion]) {
        let count = symbols.len().min(output.len());

        for i in 0..count {
            output[i] = self.encode_symbol(symbols[i]);
        }

        self.metrics.symbols_processed += count as u64;
    }

    /// Encode bytes (packing multiple bits)
    pub fn encode_bytes(&mut self, bytes: &[u8]) -> Vec<Quaternion> {
        // 7 bits per symbol, so pack accordingly
        // Each 7 bytes becomes 8 symbols (56 bits)
        let mut symbols = Vec::with_capacity((bytes.len() * 8 + 6) / 7);

        let mut bit_buffer = 0u64;
        let mut bits_in_buffer = 0;

        for &byte in bytes {
            bit_buffer |= (byte as u64) << bits_in_buffer;
            bits_in_buffer += 8;

            while bits_in_buffer >= 7 {
                symbols.push((bit_buffer & 0x7F) as u8);
                bit_buffer >>= 7;
                bits_in_buffer -= 7;
            }
        }

        // Flush remaining bits
        if bits_in_buffer > 0 {
            symbols.push((bit_buffer & 0x7F) as u8);
        }

        self.encode_batch(&symbols)
    }

    /// Get performance metrics
    pub fn metrics(&self) -> &PerformanceMetrics {
        &self.metrics
    }

    /// Reset metrics
    pub fn reset_metrics(&mut self) {
        self.metrics.reset();
    }
}

impl Default for BatchEncoder {
    fn default() -> Self {
        Self::new(BatchConfig::default())
    }
}

/// High-performance batch decoder
pub struct BatchDecoder {
    config: BatchConfig,
    simd_voronoi: SimdVoronoi,
    gray_mapper: GrayCodeMapper,
    /// Current lattice rotation (inverse for decoding)
    rotation_inv: Quaternion,
    /// Error threshold for valid reception
    error_threshold: f64,
    /// Metrics
    metrics: PerformanceMetrics,
}

impl BatchDecoder {
    /// Create new batch decoder
    pub fn new(config: BatchConfig) -> Self {
        let hex = Hexacosichoron::new();

        Self {
            config,
            simd_voronoi: SimdVoronoi::new(&hex),
            gray_mapper: GrayCodeMapper::new(&hex),
            rotation_inv: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            error_threshold: 0.3,
            metrics: PerformanceMetrics::default(),
        }
    }

    /// Set lattice rotation for decoding
    pub fn set_rotation(&mut self, rotation: Quaternion) {
        self.rotation_inv = rotation.conjugate();
    }

    /// Set error threshold
    pub fn set_error_threshold(&mut self, threshold: f64) {
        self.error_threshold = threshold;
    }

    /// Decode a single quaternion
    #[inline]
    pub fn decode_symbol(&self, q: &Quaternion) -> (u8, f64) {
        // Remove lattice rotation
        let compensated = self.rotation_inv * *q * self.rotation_inv.conjugate();
        let normalized = compensated.normalize();

        // Find nearest vertex
        let vertex_idx = self.simd_voronoi.nearest(&normalized);
        let nearest = self.simd_voronoi.vertex(vertex_idx);
        let distance = normalized.distance(&nearest);

        // Map vertex to bits
        let bits = self.gray_mapper.decode(vertex_idx).unwrap_or(0);

        (bits, distance)
    }

    /// Decode batch of quaternions
    pub fn decode_batch(&mut self, received: &[Quaternion]) -> Vec<DecodedSymbol> {
        let start = std::time::Instant::now();

        let mut results = Vec::with_capacity(received.len());

        // Process in groups of 4 for SIMD efficiency
        for chunk in received.chunks(4) {
            if chunk.len() == 4 {
                // Compensate rotation
                let compensated: [Quaternion; 4] = [
                    (self.rotation_inv * chunk[0] * self.rotation_inv.conjugate()).normalize(),
                    (self.rotation_inv * chunk[1] * self.rotation_inv.conjugate()).normalize(),
                    (self.rotation_inv * chunk[2] * self.rotation_inv.conjugate()).normalize(),
                    (self.rotation_inv * chunk[3] * self.rotation_inv.conjugate()).normalize(),
                ];

                // SIMD batch lookup
                let indices = self.simd_voronoi.nearest_4(&compensated);

                for (i, &idx) in indices.iter().enumerate() {
                    let nearest = self.simd_voronoi.vertex(idx);
                    let distance = compensated[i].distance(&nearest);
                    let bits = self.gray_mapper.decode(idx).unwrap_or(0);
                    let valid = distance < self.error_threshold;

                    results.push(DecodedSymbol {
                        bits,
                        vertex_index: idx,
                        distance,
                        valid,
                    });
                }
            } else {
                // Handle remainder
                for q in chunk {
                    let (bits, distance) = self.decode_symbol(q);
                    let idx = self.simd_voronoi.nearest(q);

                    results.push(DecodedSymbol {
                        bits,
                        vertex_index: idx,
                        distance,
                        valid: distance < self.error_threshold,
                    });
                }
            }
        }

        let elapsed = start.elapsed().as_nanos() as u64;
        self.metrics.symbols_processed += received.len() as u64;
        self.metrics.total_time_ns += elapsed;
        self.metrics.simd_operations += (received.len() / 4) as u64;
        self.metrics.fallback_operations += (received.len() % 4) as u64;

        results
    }

    /// Decode batch to bytes
    pub fn decode_to_bytes(&mut self, received: &[Quaternion]) -> (Vec<u8>, usize) {
        let decoded = self.decode_batch(received);

        // Unpack 7-bit symbols to bytes
        let mut bytes = Vec::with_capacity((decoded.len() * 7 + 7) / 8);
        let mut bit_buffer = 0u64;
        let mut bits_in_buffer = 0;
        let mut errors = 0;

        for sym in &decoded {
            if !sym.valid {
                errors += 1;
            }

            bit_buffer |= (sym.bits as u64) << bits_in_buffer;
            bits_in_buffer += 7;

            while bits_in_buffer >= 8 {
                bytes.push((bit_buffer & 0xFF) as u8);
                bit_buffer >>= 8;
                bits_in_buffer -= 8;
            }
        }

        (bytes, errors)
    }

    /// Get performance metrics
    pub fn metrics(&self) -> &PerformanceMetrics {
        &self.metrics
    }

    /// Reset metrics
    pub fn reset_metrics(&mut self) {
        self.metrics.reset();
    }
}

impl Default for BatchDecoder {
    fn default() -> Self {
        Self::new(BatchConfig::default())
    }
}

/// Decoded symbol with metadata
#[derive(Clone, Debug)]
pub struct DecodedSymbol {
    /// Decoded bits (7 bits)
    pub bits: u8,
    /// Nearest vertex index
    pub vertex_index: usize,
    /// Distance to nearest vertex
    pub distance: f64,
    /// Whether within error threshold
    pub valid: bool,
}

/// Buffer pool for zero-copy operations
pub struct BufferPool {
    /// Available quaternion buffers
    quaternion_buffers: Vec<Vec<Quaternion>>,
    /// Available byte buffers
    byte_buffers: Vec<Vec<u8>>,
    /// Buffer size
    buffer_size: usize,
}

impl BufferPool {
    /// Create new buffer pool
    pub fn new(pool_size: usize, buffer_size: usize) -> Self {
        let quaternion_buffers = (0..pool_size)
            .map(|_| Vec::with_capacity(buffer_size))
            .collect();

        let byte_buffers = (0..pool_size)
            .map(|_| Vec::with_capacity(buffer_size))
            .collect();

        Self {
            quaternion_buffers,
            byte_buffers,
            buffer_size,
        }
    }

    /// Get a quaternion buffer
    pub fn get_quaternion_buffer(&mut self) -> Option<Vec<Quaternion>> {
        self.quaternion_buffers.pop()
    }

    /// Return a quaternion buffer
    pub fn return_quaternion_buffer(&mut self, mut buffer: Vec<Quaternion>) {
        buffer.clear();
        if buffer.capacity() >= self.buffer_size {
            self.quaternion_buffers.push(buffer);
        }
    }

    /// Get a byte buffer
    pub fn get_byte_buffer(&mut self) -> Option<Vec<u8>> {
        self.byte_buffers.pop()
    }

    /// Return a byte buffer
    pub fn return_byte_buffer(&mut self, mut buffer: Vec<u8>) {
        buffer.clear();
        if buffer.capacity() >= self.buffer_size {
            self.byte_buffers.push(buffer);
        }
    }

    /// Available quaternion buffers
    pub fn available_quaternion_buffers(&self) -> usize {
        self.quaternion_buffers.len()
    }

    /// Available byte buffers
    pub fn available_byte_buffers(&self) -> usize {
        self.byte_buffers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_encoder_single() {
        let encoder = BatchEncoder::default();

        for symbol in 0..120 {
            let encoded = encoder.encode_symbol(symbol as u8);
            assert!(encoded.is_normalized(), "Symbol {} not normalized", symbol);
        }
    }

    #[test]
    fn test_batch_encoder_batch() {
        let mut encoder = BatchEncoder::default();
        let symbols: Vec<u8> = (0..64).collect();

        let encoded = encoder.encode_batch(&symbols);

        assert_eq!(encoded.len(), 64);
        for q in &encoded {
            assert!(q.is_normalized());
        }
    }

    #[test]
    fn test_batch_decoder_single() {
        let encoder = BatchEncoder::default();
        let decoder = BatchDecoder::default();

        // Encode and decode
        for symbol in 0..120 {
            let encoded = encoder.encode_symbol(symbol as u8);
            let (decoded, distance) = decoder.decode_symbol(&encoded);

            assert_eq!(decoded, symbol as u8, "Symbol {} decoded as {}", symbol, decoded);
            assert!(distance < 0.01, "Symbol {} distance {}", symbol, distance);
        }
    }

    #[test]
    fn test_batch_roundtrip() {
        let mut encoder = BatchEncoder::default();
        let mut decoder = BatchDecoder::default();

        let symbols: Vec<u8> = (0..64).collect();
        let encoded = encoder.encode_batch(&symbols);
        let decoded = decoder.decode_batch(&encoded);

        for (i, sym) in decoded.iter().enumerate() {
            assert_eq!(sym.bits, symbols[i], "Symbol {} mismatch", i);
            assert!(sym.valid, "Symbol {} invalid", i);
        }
    }

    #[test]
    fn test_bytes_roundtrip() {
        let mut encoder = BatchEncoder::default();
        let mut decoder = BatchDecoder::default();

        let original = b"Hello, CSPM!";
        let encoded = encoder.encode_bytes(original);
        let (decoded, errors) = decoder.decode_to_bytes(&encoded);

        assert_eq!(errors, 0);
        assert_eq!(&decoded[..original.len()], original);
    }

    #[test]
    fn test_batch_with_rotation() {
        let mut encoder = BatchEncoder::default();
        let mut decoder = BatchDecoder::default();

        let rotation = Quaternion::from_axis_angle([0.0, 1.0, 0.0], 0.5);
        encoder.set_rotation(rotation);
        decoder.set_rotation(rotation);

        let symbols: Vec<u8> = (0..32).collect();
        let encoded = encoder.encode_batch(&symbols);
        let decoded = decoder.decode_batch(&encoded);

        for (i, sym) in decoded.iter().enumerate() {
            assert_eq!(sym.bits, symbols[i], "Symbol {} mismatch with rotation", i);
        }
    }

    #[test]
    fn test_buffer_pool() {
        let mut pool = BufferPool::new(4, 64);

        // Get buffers
        let buf1 = pool.get_quaternion_buffer();
        let buf2 = pool.get_quaternion_buffer();

        assert!(buf1.is_some());
        assert!(buf2.is_some());
        assert_eq!(pool.available_quaternion_buffers(), 2);

        // Return buffers
        pool.return_quaternion_buffer(buf1.unwrap());
        assert_eq!(pool.available_quaternion_buffers(), 3);
    }

    #[test]
    fn test_metrics() {
        let mut encoder = BatchEncoder::default();
        let symbols: Vec<u8> = (0..100).collect();

        encoder.encode_batch(&symbols);

        let metrics = encoder.metrics();
        assert_eq!(metrics.symbols_processed, 100);
        assert!(metrics.total_time_ns > 0);
    }

    #[test]
    fn test_batch_result_throughput() {
        let result = BatchResult {
            count: 1000,
            time_ns: 1_000_000, // 1ms
            errors: 0,
        };

        let throughput = result.throughput();
        // 1000 symbols / 1ms = 1,000,000 symbols/sec
        assert!((throughput - 1_000_000.0).abs() < 1.0);
    }
}
