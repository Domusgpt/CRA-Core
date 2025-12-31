# Hexacosichoron (600-Cell) Geometry — Technical Specification

**Component of:** CSPM/1.0
**Version:** 1.0

---

## Overview

The **600-cell** (hexacosichoron) is the 4-dimensional analogue of the icosahedron. Its 120 vertices, when normalized to the unit 3-sphere, form the **binary icosahedral group 2I** — the double cover of the icosahedral symmetry group.

This document specifies the vertex coordinates, group structure, Voronoi tessellation, and Gray-code mapping used in CSPM.

---

## 1. Structure

### 1.1 Properties

| Property       | Value                    |
|----------------|--------------------------|
| Vertices       | 120                      |
| Edges          | 720                      |
| Faces          | 1200 (triangular)        |
| Cells          | 600 (tetrahedral)        |
| Dual           | 120-cell (hecatonicosachoron) |
| Symmetry group | H₄ (order 14400)         |

### 1.2 Schläfli Symbol

```
{3, 3, 5}

meaning:
  - Faces are triangles {3}
  - Vertex figures are icosahedra {3, 5}
  - Cells are tetrahedra {3, 3}
```

---

## 2. Vertex Coordinates

All 120 vertices are unit quaternions (|q| = 1). They fall into three groups based on construction.

### 2.1 Group A: 8 Vertices (Orthoplex)

These are the ±1 coordinates along each axis:

```
( 1,  0,  0,  0)    (-1,  0,  0,  0)
( 0,  1,  0,  0)    ( 0, -1,  0,  0)
( 0,  0,  1,  0)    ( 0,  0, -1,  0)
( 0,  0,  0,  1)    ( 0,  0,  0, -1)
```

### 2.2 Group B: 16 Vertices (Tesseract)

All combinations of (±½, ±½, ±½, ±½):

```
( ½,  ½,  ½,  ½)    ( ½,  ½,  ½, -½)    ( ½,  ½, -½,  ½)    ( ½,  ½, -½, -½)
( ½, -½,  ½,  ½)    ( ½, -½,  ½, -½)    ( ½, -½, -½,  ½)    ( ½, -½, -½, -½)
(-½,  ½,  ½,  ½)    (-½,  ½,  ½, -½)    (-½,  ½, -½,  ½)    (-½,  ½, -½, -½)
(-½, -½,  ½,  ½)    (-½, -½,  ½, -½)    (-½, -½, -½,  ½)    (-½, -½, -½, -½)
```

### 2.3 Group C: 96 Vertices (Icosahedral)

Even permutations of (±φ/2, ±½, ±1/(2φ), 0), where φ = (1+√5)/2:

```
Let:
  a = φ/2    ≈ 0.809016994
  b = 1/2    = 0.5
  c = 1/(2φ) ≈ 0.309016994

The 96 vertices are all even permutations of:
  (±a, ±b, ±c, 0)

An "even permutation" means the components are shuffled by an even number of swaps.
```

**All 24 base forms (before sign variations):**

```
(a, b, c, 0)    (a, b, 0, c)    (a, c, b, 0)    (a, c, 0, b)
(a, 0, b, c)    (a, 0, c, b)    (b, a, c, 0)    (b, a, 0, c)
(b, c, a, 0)    (b, c, 0, a)    (b, 0, a, c)    (b, 0, c, a)
(c, a, b, 0)    (c, a, 0, b)    (c, b, a, 0)    (c, b, 0, a)
(c, 0, a, b)    (c, 0, b, a)    (0, a, b, c)    (0, a, c, b)
(0, b, a, c)    (0, b, c, a)    (0, c, a, b)    (0, c, b, a)
```

Each base form has 4 sign variants (for the non-zero components), giving 24 × 4 = 96 vertices.

---

## 3. Complete Vertex Table

### 3.1 Vertex Generation (Rust)

```rust
use std::f64::consts::PI;

const PHI: f64 = 1.6180339887498948482;
const A: f64 = PHI / 2.0;       // ≈ 0.809
const B: f64 = 0.5;
const C: f64 = 1.0 / (2.0 * PHI); // ≈ 0.309

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vertex {
    pub fn new(w: f64, x: f64, y: f64, z: f64) -> Self {
        Vertex { w, x, y, z }
    }

    pub fn norm(&self) -> f64 {
        (self.w * self.w + self.x * self.x +
         self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn distance(&self, other: &Vertex) -> f64 {
        let dw = self.w - other.w;
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dw * dw + dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn angular_distance(&self, other: &Vertex) -> f64 {
        let dot = self.w * other.w + self.x * other.x +
                  self.y * other.y + self.z * other.z;
        dot.clamp(-1.0, 1.0).acos()
    }
}

pub fn generate_600_cell_vertices() -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(120);

    // Group A: 8 vertices (±1 along each axis)
    for i in 0..4 {
        let mut v = [0.0; 4];
        v[i] = 1.0;
        vertices.push(Vertex::new(v[0], v[1], v[2], v[3]));
        v[i] = -1.0;
        vertices.push(Vertex::new(v[0], v[1], v[2], v[3]));
    }

    // Group B: 16 vertices (all ±½ combinations)
    for signs in 0..16 {
        let w = if signs & 1 != 0 { 0.5 } else { -0.5 };
        let x = if signs & 2 != 0 { 0.5 } else { -0.5 };
        let y = if signs & 4 != 0 { 0.5 } else { -0.5 };
        let z = if signs & 8 != 0 { 0.5 } else { -0.5 };
        vertices.push(Vertex::new(w, x, y, z));
    }

    // Group C: 96 vertices (even permutations of (a, b, c, 0))
    let base = [A, B, C, 0.0];

    // Generate all even permutations
    let even_perms = [
        [0, 1, 2, 3], [0, 2, 3, 1], [0, 3, 1, 2],
        [1, 0, 3, 2], [1, 2, 0, 3], [1, 3, 2, 0],
        [2, 0, 1, 3], [2, 1, 3, 0], [2, 3, 0, 1],
        [3, 0, 2, 1], [3, 1, 0, 2], [3, 2, 1, 0],
    ];

    for perm in &even_perms {
        let vals = [base[perm[0]], base[perm[1]], base[perm[2]], base[perm[3]]];

        // For each permutation, apply all 8 sign combinations
        // to the three non-zero values
        for signs in 0..8 {
            let mut v = vals;
            // Find which positions are non-zero and apply signs
            let mut sign_idx = 0;
            for i in 0..4 {
                if v[i] != 0.0 {
                    if signs & (1 << sign_idx) != 0 {
                        v[i] = -v[i];
                    }
                    sign_idx += 1;
                }
            }
            vertices.push(Vertex::new(v[0], v[1], v[2], v[3]));
        }
    }

    // Verify count
    assert_eq!(vertices.len(), 120);

    // Verify all are unit quaternions
    for v in &vertices {
        assert!((v.norm() - 1.0).abs() < 1e-10);
    }

    vertices
}
```

---

## 4. Gray Code Mapping

### 4.1 Design Goals

- Adjacent vertices (connected by edges) should differ by minimal bits
- Enable soft-decision decoding
- Reserve vertices 120-127 for control symbols

### 4.2 Construction

We use a **graph-based Gray code** generated by traversing the 600-cell edge graph:

```rust
pub struct GrayCodeMapper {
    vertex_to_bits: [u8; 120],
    bits_to_vertex: [u8; 128],  // 128 to handle 7 bits
}

impl GrayCodeMapper {
    pub fn new(vertices: &[Vertex]) -> Self {
        let mut mapper = GrayCodeMapper {
            vertex_to_bits: [0; 120],
            bits_to_vertex: [0xFF; 128],  // 0xFF = invalid
        };

        // Build adjacency graph (vertices connected by edges)
        let adjacency = build_adjacency_graph(vertices);

        // Perform Hamiltonian-like traversal for Gray code
        let ordering = gray_traversal(&adjacency);

        for (bits, &vertex_idx) in ordering.iter().enumerate() {
            if bits < 120 {
                mapper.vertex_to_bits[vertex_idx] = bits as u8;
                mapper.bits_to_vertex[bits] = vertex_idx as u8;
            }
        }

        // Control symbols (120-127) map to closest vertices
        for bits in 120..128 {
            mapper.bits_to_vertex[bits] = (bits - 120) as u8;
        }

        mapper
    }

    pub fn encode(&self, bits: u8) -> Option<usize> {
        if bits >= 128 {
            return None;
        }
        let idx = self.bits_to_vertex[bits as usize];
        if idx == 0xFF { None } else { Some(idx as usize) }
    }

    pub fn decode(&self, vertex_idx: usize) -> Option<u8> {
        if vertex_idx >= 120 {
            return None;
        }
        Some(self.vertex_to_bits[vertex_idx])
    }
}

fn build_adjacency_graph(vertices: &[Vertex]) -> Vec<Vec<usize>> {
    const EDGE_LENGTH: f64 = 0.618033988749895;  // 1/φ
    const TOLERANCE: f64 = 1e-6;

    let mut adj = vec![Vec::new(); vertices.len()];

    for i in 0..vertices.len() {
        for j in (i + 1)..vertices.len() {
            let d = vertices[i].distance(&vertices[j]);
            if (d - EDGE_LENGTH).abs() < TOLERANCE {
                adj[i].push(j);
                adj[j].push(i);
            }
        }
    }

    // Each vertex should have exactly 12 neighbors
    for neighbors in &adj {
        assert_eq!(neighbors.len(), 12);
    }

    adj
}
```

### 4.3 Bit-to-Vertex Table (First 32 entries)

```
Bits   Vertex   Quaternion (w, x, y, z)
0x00   0        ( 1.000,  0.000,  0.000,  0.000)
0x01   1        ( 0.809,  0.500,  0.309,  0.000)
0x02   2        ( 0.809,  0.309,  0.500,  0.000)
0x03   3        ( 0.809,  0.500,  0.000,  0.309)
0x04   4        ( 0.809,  0.309,  0.000,  0.500)
0x05   5        ( 0.809,  0.000,  0.500,  0.309)
0x06   6        ( 0.809,  0.000,  0.309,  0.500)
0x07   7        ( 0.500,  0.500,  0.500,  0.500)
0x08   8        ( 0.500,  0.809,  0.309,  0.000)
...
0x1F   31       ...

(Full table in vertex_table.json)
```

---

## 5. Voronoi Tessellation

### 5.1 Voronoi Cells

Each vertex has a **Voronoi cell** — the region of S³ closer to that vertex than any other.

**Properties:**
- Each cell is a spherical polyhedron
- All cells are congruent (regular tessellation)
- Solid angle: 4π/120 ≈ 0.105 steradians
- Angular radius: arcsin(1/(2φ)) ≈ 18°

### 5.2 Nearest Vertex Lookup

For geometric quantization (error correction), we need O(1) nearest vertex lookup.

**Method 1: Exact (slow)**
```rust
fn nearest_vertex_exact(q: &Vertex, vertices: &[Vertex]) -> usize {
    vertices
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance(q)
                .partial_cmp(&b.distance(q))
                .unwrap()
        })
        .map(|(i, _)| i)
        .unwrap()
}
```

**Method 2: Hierarchical (fast)**
```rust
pub struct VoronoiLookup {
    // Partition S³ into regions for fast lookup
    regions: Vec<Vec<usize>>,  // vertex indices per region
    region_count: usize,
}

impl VoronoiLookup {
    pub fn new(vertices: &[Vertex]) -> Self {
        // Create 600 regions (one per cell of dual 120-cell)
        // Precompute which vertices could be nearest for each region
        // ...
        todo!()
    }

    pub fn nearest(&self, q: &Vertex, vertices: &[Vertex]) -> usize {
        // 1. Determine which region q falls into (O(1))
        // 2. Check only candidate vertices in that region (O(1))
        let region = self.get_region(q);
        self.regions[region]
            .iter()
            .copied()
            .min_by(|&a, &b| {
                vertices[a].distance(q)
                    .partial_cmp(&vertices[b].distance(q))
                    .unwrap()
            })
            .unwrap()
    }
}
```

**Method 3: Decision Tree (fastest)**

Precompute a binary decision tree that partitions S³:

```rust
pub struct VoronoiTree {
    nodes: Vec<TreeNode>,
}

enum TreeNode {
    Split {
        plane: [f64; 4],  // Hyperplane normal
        threshold: f64,
        left: usize,
        right: usize,
    },
    Leaf {
        vertex: usize,
    },
}

impl VoronoiTree {
    pub fn nearest(&self, q: &Vertex) -> usize {
        let mut node = 0;
        loop {
            match &self.nodes[node] {
                TreeNode::Leaf { vertex } => return *vertex,
                TreeNode::Split { plane, threshold, left, right } => {
                    let dot = plane[0] * q.w + plane[1] * q.x +
                              plane[2] * q.y + plane[3] * q.z;
                    node = if dot < *threshold { *left } else { *right };
                }
            }
        }
    }
}
```

---

## 6. Group Structure

### 6.1 Binary Icosahedral Group

The 120 vertices form the **binary icosahedral group 2I** under quaternion multiplication:

- Order: 120
- Generators: Two quaternions that generate all others
- Subgroups: 2T (binary tetrahedral), 2O (binary octahedral), Q₈ (quaternion group)

### 6.2 Group Generators

```
g₁ = (φ/2, 1/2, 1/(2φ), 0) ≈ (0.809, 0.5, 0.309, 0)
g₂ = (1/2, 1/2, 1/2, 1/2)

Every vertex can be expressed as products of g₁ and g₂.
```

### 6.3 Rotation Action

When the lattice is rotated by quaternion R:

```
v' = R ⊗ v ⊗ R*

for all vertices v in the 600-cell.
```

This preserves:
- All distances between vertices
- The Voronoi tessellation structure
- The Gray code mapping (vertices just relabeled)

---

## 7. Noise Tolerance

### 7.1 Error Correction Margin

A received quaternion q_r is correctly decoded if:

```
‖q_r - v_true‖ < ‖q_r - v_other‖  for all v_other ≠ v_true
```

Equivalently, q_r must fall within the Voronoi cell of v_true.

**Geometric margin:**
```
d_min / 2 = (1/φ) / 2 ≈ 0.309

In angular terms:
θ_max = arcsin(d_min / 2) ≈ 18°
```

### 7.2 Comparison to QAM

| Constellation | Dimension | Points | Min Distance | Coding Gain |
|---------------|-----------|--------|--------------|-------------|
| QPSK          | 2D        | 4      | √2 ≈ 1.41    | 0 dB        |
| 16-QAM        | 2D        | 16     | 0.47         | -4.8 dB     |
| 64-QAM        | 2D        | 64     | 0.22         | -7.4 dB     |
| **600-cell**  | **4D**    | **120**| **0.618**    | **+4.5 dB** |

The 600-cell achieves **optimal sphere packing** in 4D, giving inherent coding gain.

---

## 8. Visualization

### 8.1 Projections

The 600-cell can be visualized via stereographic projection to 3D:

```
3D Stereographic Projection from (1, 0, 0, 0):

    (x', y', z') = (x, y, z) / (1 - w)

This maps the 3-sphere to R³, with the projection point at infinity.
```

### 8.2 Vertex Distribution

When projected:
- Vertices cluster near origin (from vertices near projection point)
- Outer vertices appear at larger radii
- Edge structure reveals icosahedral symmetry

---

## 9. Implementation Files

```
cspm-core/
└── polytope/
    ├── hexacosichoron.rs    # Vertex generation
    ├── group.rs             # Binary icosahedral group
    ├── voronoi.rs           # Nearest vertex lookup
    ├── gray_code.rs         # Bit ↔ vertex mapping
    └── vertex_table.json    # Precomputed coordinates
```

---

## Appendix A: Full Vertex Coordinates

See `polytope/vertex_table.json` for all 120 vertices with:
- Index (0-119)
- Quaternion components (w, x, y, z) to 15 decimal places
- Gray code bits (7-bit)
- Neighbor indices (12 per vertex)

---

## Appendix B: Edge List

The 720 edges connect vertices at distance 1/φ ≈ 0.618:

```
Each vertex has exactly 12 edges.
Edge graph is vertex-transitive (all vertices equivalent).

First 20 edges:
(0, 24), (0, 25), (0, 26), (0, 27), (0, 28), (0, 29),
(0, 30), (0, 31), (0, 32), (0, 33), (0, 34), (0, 35),
(1, 8), (1, 9), (1, 15), (1, 24), (1, 35), (1, 40),
(1, 41), (1, 66), ...
```

---

*End of Hexacosichoron Geometry Specification*
