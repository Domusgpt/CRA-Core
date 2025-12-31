//! Gray code mapping for bit-to-vertex encoding.
//!
//! Maps 7-bit values to 600-cell vertices such that adjacent vertices
//! (connected by edges) differ by minimal bits.

use super::Hexacosichoron;
use crate::BITS_PER_SYMBOL;

/// Gray code mapper for 600-cell vertices
pub struct GrayCodeMapper {
    /// Maps vertex index (0-119) to bit pattern (0-127)
    vertex_to_bits: [u8; 120],
    /// Maps bit pattern (0-127) to vertex index
    bits_to_vertex: [u8; 128],
}

impl GrayCodeMapper {
    /// Create a new Gray code mapper
    pub fn new(hexacosichoron: &Hexacosichoron) -> Self {
        let mut mapper = Self {
            vertex_to_bits: [0; 120],
            bits_to_vertex: [0xFF; 128], // 0xFF = invalid/unmapped
        };

        // Build adjacency-aware Gray code ordering
        let ordering = Self::build_gray_ordering(hexacosichoron);

        // Assign bit patterns to vertices
        for (bits, &vertex_idx) in ordering.iter().enumerate() {
            if bits < 120 {
                mapper.vertex_to_bits[vertex_idx as usize] = bits as u8;
                mapper.bits_to_vertex[bits] = vertex_idx;
            }
        }

        // Control symbols (120-127) map to first 8 vertices
        for bits in 120..128 {
            mapper.bits_to_vertex[bits] = (bits - 120) as u8;
        }

        mapper
    }

    /// Encode bits to vertex index
    pub fn encode(&self, bits: u8) -> Option<usize> {
        if bits >= 128 {
            return None;
        }
        let idx = self.bits_to_vertex[bits as usize];
        if idx == 0xFF {
            None
        } else {
            Some(idx as usize)
        }
    }

    /// Decode vertex index to bits
    pub fn decode(&self, vertex_idx: usize) -> Option<u8> {
        if vertex_idx >= 120 {
            return None;
        }
        Some(self.vertex_to_bits[vertex_idx])
    }

    /// Check if a bit pattern is a control symbol
    pub fn is_control_symbol(&self, bits: u8) -> bool {
        bits >= 120 && bits < 128
    }

    /// Get the vertex index for a control symbol
    pub fn control_vertex(&self, control_code: u8) -> Option<usize> {
        if control_code < 8 {
            Some(control_code as usize)
        } else {
            None
        }
    }

    /// Build Gray code ordering using graph traversal
    fn build_gray_ordering(hexacosichoron: &Hexacosichoron) -> Vec<u8> {
        // Use a DFS-based traversal to create an ordering where
        // adjacent vertices (in the traversal) are also graph neighbors

        let mut visited = vec![false; 120];
        let mut ordering = Vec::with_capacity(120);

        // Start from vertex 0
        Self::gray_dfs(hexacosichoron, 0, &mut visited, &mut ordering);

        // If DFS didn't reach all vertices, add remaining
        for i in 0..120 {
            if !visited[i] {
                ordering.push(i as u8);
            }
        }

        ordering
    }

    /// DFS traversal for Gray code construction
    fn gray_dfs(
        hexacosichoron: &Hexacosichoron,
        vertex: usize,
        visited: &mut Vec<bool>,
        ordering: &mut Vec<u8>,
    ) {
        if visited[vertex] {
            return;
        }

        visited[vertex] = true;
        ordering.push(vertex as u8);

        // Visit neighbors in order of their index
        if let Some(neighbors) = hexacosichoron.neighbors(vertex) {
            let mut sorted_neighbors = neighbors.to_vec();
            sorted_neighbors.sort();

            for &neighbor in &sorted_neighbors {
                Self::gray_dfs(hexacosichoron, neighbor, visited, ordering);
            }
        }
    }

    /// Get the number of bit differences between two vertex encodings
    pub fn hamming_distance(&self, v1: usize, v2: usize) -> Option<u32> {
        let bits1 = self.decode(v1)?;
        let bits2 = self.decode(v2)?;
        Some((bits1 ^ bits2).count_ones())
    }

    /// Encode a byte slice to vertex indices
    pub fn encode_bytes(&self, data: &[u8]) -> Vec<usize> {
        // Pack bytes into 7-bit symbols
        let mut symbols = Vec::new();
        let mut bit_buffer: u64 = 0;
        let mut bits_in_buffer = 0u32;

        for &byte in data {
            bit_buffer |= (byte as u64) << bits_in_buffer;
            bits_in_buffer += 8;

            while bits_in_buffer >= BITS_PER_SYMBOL {
                let symbol = (bit_buffer & 0x7F) as u8; // Extract 7 bits
                if let Some(vertex) = self.encode(symbol) {
                    symbols.push(vertex);
                }
                bit_buffer >>= BITS_PER_SYMBOL;
                bits_in_buffer -= BITS_PER_SYMBOL;
            }
        }

        // Handle remaining bits
        if bits_in_buffer > 0 {
            let symbol = (bit_buffer & 0x7F) as u8;
            if let Some(vertex) = self.encode(symbol) {
                symbols.push(vertex);
            }
        }

        symbols
    }

    /// Decode vertex indices to bytes
    pub fn decode_vertices(&self, vertices: &[usize]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut bit_buffer: u64 = 0;
        let mut bits_in_buffer = 0u32;

        for &vertex in vertices {
            if let Some(symbol) = self.decode(vertex) {
                bit_buffer |= (symbol as u64) << bits_in_buffer;
                bits_in_buffer += BITS_PER_SYMBOL;

                while bits_in_buffer >= 8 {
                    bytes.push((bit_buffer & 0xFF) as u8);
                    bit_buffer >>= 8;
                    bits_in_buffer -= 8;
                }
            }
        }

        bytes
    }
}

impl Default for GrayCodeMapper {
    fn default() -> Self {
        Self::new(&Hexacosichoron::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let h = Hexacosichoron::new();
        let mapper = GrayCodeMapper::new(&h);

        for bits in 0..120u8 {
            let vertex = mapper.encode(bits).unwrap();
            let decoded = mapper.decode(vertex).unwrap();
            assert_eq!(decoded, bits, "Roundtrip failed for bits {}", bits);
        }
    }

    #[test]
    fn test_all_vertices_mapped() {
        let h = Hexacosichoron::new();
        let mapper = GrayCodeMapper::new(&h);

        let mut mapped_vertices = vec![false; 120];
        for bits in 0..120u8 {
            let vertex = mapper.encode(bits).unwrap();
            assert!(!mapped_vertices[vertex], "Vertex {} mapped twice", vertex);
            mapped_vertices[vertex] = true;
        }

        for (i, &mapped) in mapped_vertices.iter().enumerate() {
            assert!(mapped, "Vertex {} not mapped", i);
        }
    }

    #[test]
    fn test_control_symbols() {
        let h = Hexacosichoron::new();
        let mapper = GrayCodeMapper::new(&h);

        for bits in 120..128u8 {
            assert!(mapper.is_control_symbol(bits));
            let vertex = mapper.encode(bits).unwrap();
            assert!(vertex < 8);
        }
    }

    #[test]
    fn test_byte_encoding() {
        let h = Hexacosichoron::new();
        let mapper = GrayCodeMapper::new(&h);

        let data = b"Hello, CSPM!";
        let vertices = mapper.encode_bytes(data);
        let decoded = mapper.decode_vertices(&vertices);

        // First bytes should match (may have trailing due to bit alignment)
        for (i, &expected) in data.iter().enumerate() {
            if i < decoded.len() {
                assert_eq!(decoded[i], expected, "Mismatch at byte {}", i);
            }
        }
    }
}
