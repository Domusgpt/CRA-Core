# CSPM System Architecture

This document describes the technical architecture of the CSPM (Cryptographically-Seeded Polytopal Modulation) system.

---

## Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           APPLICATION LAYER                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                   │
│  │   File TX    │  │   Secure     │  │    TRACE     │                   │
│  │   Service    │  │   Messaging  │  │   Telemetry  │                   │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘                   │
└─────────┼─────────────────┼─────────────────┼───────────────────────────┘
          │                 │                 │
          └────────────────┬┴─────────────────┘
                           │
┌──────────────────────────┴──────────────────────────────────────────────┐
│                           CSPM CORE LIBRARY                              │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                         modulation/                                 │ │
│  │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐            │ │
│  │  │  Encoder    │───▶│  Quaternion │───▶│   Optical   │            │ │
│  │  │  (bits→q)   │    │   Space     │    │   State     │            │ │
│  │  └─────────────┘    └─────────────┘    └─────────────┘            │ │
│  │                            ▲                  │                    │ │
│  │                            │                  ▼                    │ │
│  │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐            │ │
│  │  │  Decoder    │◀───│   Voronoi   │◀───│   Channel   │            │ │
│  │  │  (q→bits)   │    │   Lookup    │    │   Effects   │            │ │
│  │  └─────────────┘    └─────────────┘    └─────────────┘            │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  polytope/  │  │   crypto/   │  │ simulation/ │  │   trace/    │    │
│  │  600-cell   │  │  hash chain │  │   channel   │  │   events    │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         HARDWARE ABSTRACTION                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │    SLM      │  │  Polarizer  │  │    OAM      │  │  Coherent   │    │
│  │  Controller │  │   Optics    │  │   Sorter    │  │  Receiver   │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
└─────────┼────────────────┼────────────────┼────────────────┼────────────┘
          │                │                │                │
          └────────────────┴────────────────┴────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         PHYSICAL LAYER                                   │
│                    (Optical fiber / Free-space)                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. Quaternion Module (`quaternion/`)

Implements 4D quaternion mathematics for signal space representation.

```rust
pub struct Quaternion {
    pub w: f64,  // Scalar part
    pub x: f64,  // i component
    pub y: f64,  // j component
    pub z: f64,  // k component
}
```

**Key Operations:**
- Hamilton product: `q1 * q2`
- Conjugate: `q.conjugate()`
- Normalization: `q.normalize()`
- Distance: `q1.distance(&q2)`
- SLERP interpolation
- Axis-angle construction

**Invariant:** All quaternions in the signal path are unit quaternions (|q| = 1).

### 2. Polytope Module (`polytope/`)

Implements the 600-cell (Hexacosichoron) constellation geometry.

#### Vertex Generation (`vertices.rs`)

The 120 vertices are generated in three groups:

```
Group A (8 vertices):   (±1, 0, 0, 0) and permutations
Group B (16 vertices):  (±½, ±½, ±½, ±½)
Group C (96 vertices):  Golden ratio based: (0, ±1/2, ±φ/2, ±1/2φ) even permutations
```

where φ = (1 + √5) / 2 ≈ 1.618 (golden ratio)

#### Voronoi Tessellation (`voronoi.rs`)

O(1) nearest vertex lookup using spatial partitioning:

```rust
pub struct VoronoiLookup {
    grid: Vec<Vec<usize>>,  // Pre-computed vertex indices
    cell_size: f64,
}

impl VoronoiLookup {
    pub fn nearest(&self, q: &Quaternion) -> usize;
    pub fn nearest_with_distance(&self, q: &Quaternion) -> (usize, f64);
}
```

#### Gray Code Mapping (`gray_code.rs`)

Assigns 7-bit values to vertices minimizing Hamming distance for adjacent vertices:

```rust
pub struct GrayCodeMapper {
    vertex_to_bits: [u8; 120],
    bits_to_vertex: [usize; 128],  // 7 bits = 128 values
}
```

### 3. Modulation Module (`modulation/`)

#### Encoder (`encoder.rs`)

Converts bits to optical signals via quaternion intermediary:

```
Input: bytes
   │
   ▼
┌────────────────────┐
│ Gray code mapping  │  bits → vertex index
└────────┬───────────┘
         │
         ▼
┌────────────────────┐
│ Vertex lookup      │  index → base quaternion
└────────┬───────────┘
         │
         ▼
┌────────────────────┐
│ Lattice rotation   │  q' = R(hash) * q * R(hash)†
└────────┬───────────┘
         │
         ▼
┌────────────────────┐
│ Quaternion→Optical │  q → (S1, S2, S3, OAM)
└────────┬───────────┘
         │
         ▼
Output: OpticalState
```

#### Decoder (`decoder.rs`)

Reverses the encoding with geometric error correction:

```
Input: OpticalState (noisy)
   │
   ▼
┌────────────────────┐
│ Optical→Quaternion │  (S1, S2, S3, OAM) → q
└────────┬───────────┘
         │
         ▼
┌────────────────────┐
│ Inverse rotation   │  q = R(hash)† * q' * R(hash)
└────────┬───────────┘
         │
         ▼
┌────────────────────┐
│ Voronoi snap       │  q → nearest vertex (ERROR CORRECTION)
└────────┬───────────┘
         │
         ▼
┌────────────────────┐
│ Gray code decode   │  vertex → bits
└────────┬───────────┘
         │
         ▼
Output: bytes
```

#### Optical State (`optical.rs`)

```rust
pub struct OpticalState {
    pub stokes: StokesVector,  // Polarization (S1, S2, S3)
    pub oam_mode: i32,         // Orbital angular momentum mode
    pub power_dbm: f64,        // Optical power
}

pub struct StokesVector {
    pub s1: f64,  // Linear horizontal/vertical
    pub s2: f64,  // Linear ±45°
    pub s3: f64,  // Circular left/right
}
```

### 4. Crypto Module (`crypto/`)

Implements the hash chain for lattice rotation:

```rust
pub struct HashChain {
    state: ChainState,
    genesis_hash: [u8; 32],
}

pub struct ChainState {
    pub hash: [u8; 32],
    pub rotation: Quaternion,
    pub depth: u64,
}
```

**Chain Evolution:**
```
State[n+1].hash = SHA256(State[n].hash || symbol_data)
State[n+1].rotation = State[n].rotation * HashToQuaternion(SHA256(symbol_data))
```

**Security Properties:**
- Forward secrecy: Cannot recover previous rotations
- Non-repeatability: Same data produces different ciphertext at different positions
- Synchronization required: Receiver must track chain state

### 5. Simulation Module (`simulation/`)

#### Channel Model (`mod.rs`)

```rust
pub struct ChannelModel {
    pub fiber: Option<FiberParameters>,
    pub freespace: Option<FreespaceParameters>,
    pub oam_crosstalk: OamCrosstalkModel,
    pub hardware: HardwareModel,
    pub noise: NoiseModel,
}
```

#### Fiber Channel (`fiber.rs`)

Models SMF-28 fiber effects:
- **PMD:** Random SU(2) rotation, variance ∝ √L
- **Chromatic Dispersion:** Phase distortion, D = 17 ps/(nm·km)
- **Kerr Nonlinearity:** SPM phase shift, γ = 1.3 W⁻¹km⁻¹
- **ASE Noise:** EDFA spontaneous emission, NF = 5 dB

#### Free-Space Optical (`freespace.rs`)

Models atmospheric effects:
- **Scintillation:** Log-normal amplitude fluctuation
- **Phase Distortion:** Rytov approximation for weak turbulence
- **Beam Wander:** Pointing error accumulation
- **OAM Coupling:** Spiral phase distortion

#### Hardware Impairments (`hardware.rs`)

- **SLM:** Phase quantization (8-12 bit), flicker, fill factor
- **Detector:** Shot noise, thermal noise, dark current
- **ADC:** Quantization noise, ENOB degradation

#### Monte Carlo Simulator (`monte_carlo.rs`)

```rust
pub struct BerSimulator {
    config: SimulatorConfig,
    secret: Vec<u8>,
}

impl BerSimulator {
    pub fn simulate_point(&self, snr_db: f64, channel: &ChannelModel) -> BerPoint;
    pub fn simulate_curve(&self, snr_range: &[f64], channel: &ChannelModel) -> BerCurve;
}
```

---

## Data Flow

### Encoding Pipeline

```
1. Application provides bytes
2. Bytes split into 7-bit symbols (with padding)
3. Each symbol:
   a. Map to vertex index via Gray code
   b. Look up vertex quaternion
   c. Apply hash-chain rotation
   d. Convert to Stokes + OAM
   e. Advance hash chain with symbol
4. OpticalState array ready for transmission
```

### Decoding Pipeline

```
1. Receive OpticalState from channel
2. Each symbol:
   a. Convert Stokes + OAM to quaternion
   b. Apply inverse hash-chain rotation
   c. Voronoi snap to nearest vertex (ERROR CORRECTION HERE)
   d. Map vertex to bits via Gray code
   e. Advance hash chain with decoded bits
3. Reassemble bytes from symbols
```

### Error Correction Geometry

The 600-cell provides natural error correction:

```
           Vertex Vi
              ●
             /│\
            / │ \
           /  │  \
          /   │   \
         /    │    \
        ●─────┼─────●  Adjacent vertices Vj
         \    │    /
          \   │   /
           \  │  /
            \ │ /
             \│/
              ●

Received point anywhere in Vi's Voronoi cell
→ Snaps to Vi (correct decode)

d_min = 1/φ ≈ 0.618
Correction radius = d_min/2 ≈ 0.309
```

---

## Security Architecture

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Eavesdropping | Lattice orientation unknown without genesis hash |
| Replay attack | Rolling hash makes each symbol unique |
| Man-in-middle | Shared secret required for sync |
| Traffic analysis | All symbols appear uniform on S³ |
| Side channel | Hash chain is constant-time |

### Key Hierarchy

```
Shared Secret (out of band)
        │
        ▼
    SHA256
        │
        ▼
 Genesis Hash (32 bytes)
        │
        ├──────────────────────┐
        ▼                      ▼
 Initial Rotation      Initial Chain State
   (quaternion)            (hash)
        │                      │
        └──────────┬───────────┘
                   │
                   ▼ (per symbol)
              Hash Chain Advance
                   │
                   ├──▶ New Rotation
                   │
                   └──▶ New Hash
```

---

## Performance Characteristics

### Theoretical Limits

| Metric | Value | Notes |
|--------|-------|-------|
| Bits per symbol | 6.907 | log₂(120) |
| Symbol rate | Limited by SLM | 60-200 Hz typical |
| Parallelization | Per wavelength | WDM compatible |
| Latency | O(1) per symbol | Voronoi lookup |

### Measured Performance

| Operation | Time | Notes |
|-----------|------|-------|
| Quaternion multiply | ~10 ns | 4 multiplies + 3 adds |
| Voronoi lookup | ~50 ns | Grid-based |
| SHA256 | ~200 ns | Per advance |
| Full encode | ~500 ns | Symbol |
| Full decode | ~500 ns | Symbol |

### BER Performance (AWGN)

| SNR (dB) | BER |
|----------|-----|
| 15 | ~0.4 |
| 18 | ~0.01 |
| 20 | <10⁻³ |
| 25 | ~0 |

---

## Extension Points

### Adding New Channel Models

```rust
impl ChannelModel {
    pub fn apply(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        // Add new effect here
        let result = self.apply_new_effect(&result, rng);
        result.normalize()
    }
}
```

### Custom Constellations

The architecture supports alternative polytopes:

```rust
trait Constellation {
    fn vertices(&self) -> &[Quaternion];
    fn nearest(&self, q: &Quaternion) -> usize;
    fn bits_per_symbol(&self) -> u32;
}
```

Candidates:
- 24-cell (24 vertices, 4.6 bits)
- 120-cell (600 vertices, 9.2 bits)
- Custom optimized arrangements

### Hardware Backends

```rust
trait HardwareEncoder {
    fn encode(&self, state: &OpticalState) -> Result<(), Error>;
}

trait HardwareDecoder {
    fn decode(&self) -> Result<OpticalState, Error>;
}
```

---

## Testing Strategy

### Unit Tests

Each module has comprehensive unit tests:
- `quaternion`: Algebraic properties
- `polytope`: Vertex geometry, adjacency
- `modulation`: Encode/decode roundtrip
- `crypto`: Chain determinism
- `simulation`: Physical model validation

### Integration Tests

- Full encode→channel→decode pipeline
- Multi-symbol sequences
- Hash chain synchronization

### Property-Based Tests

- Arbitrary input bytes → successful decode
- Noise within threshold → correct decode
- Noise beyond threshold → graceful failure

---

## Dependencies

```toml
[dependencies]
sha2 = "0.10"        # Hash chain
rand = "0.8"         # RNG
rand_distr = "0.4"   # Distributions
rand_chacha = "0.3"  # Deterministic RNG
serde = "1.0"        # Serialization
chrono = "0.4"       # Timestamps
thiserror = "1.0"    # Error handling
hex = "0.4"          # Hash display
```

---

*Architecture version: 1.0*
*Last updated: 2025*
