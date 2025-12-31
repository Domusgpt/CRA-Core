# Spinor Modulation — Technical Specification

**Component of:** CSPM/1.0
**Version:** 1.0

---

## Overview

Spinor Modulation encodes information in the **full rotation group SO(4)** by utilizing both polarization (spin angular momentum) and orbital angular momentum (OAM) of photons simultaneously. This document provides the mathematical foundations and implementation details.

---

## 1. Mathematical Foundations

### 1.1 Quaternions as Spinors

A unit quaternion q ∈ H¹ (the 3-sphere S³) represents a rotation in 3D or equivalently a point in 4D:

```
q = w + xi + yj + zk

where:
  w, x, y, z ∈ ℝ
  w² + x² + y² + z² = 1
  i² = j² = k² = ijk = -1
```

The quaternion algebra is isomorphic to the **Spin(3)** group, the double cover of SO(3).

### 1.2 Quaternion Operations

**Multiplication:**
```
q₁ ⊗ q₂ = (w₁w₂ - x₁x₂ - y₁y₂ - z₁z₂,
           w₁x₂ + x₁w₂ + y₁z₂ - z₁y₂,
           w₁y₂ - x₁z₂ + y₁w₂ + z₁x₂,
           w₁z₂ + x₁y₂ - y₁x₂ + z₁w₂)
```

**Conjugate:**
```
q* = w - xi - yj - zk
```

**Rotation of vector v = (0, vₓ, vᵧ, vᵤ):**
```
v' = q ⊗ v ⊗ q*
```

### 1.3 The Hopf Fibration

The unit quaternions S³ fiber over the 2-sphere (Poincaré sphere) via the Hopf map:

```
π: S³ → S²
π(q) = (2(xy + wz), 2(yz - wx), w² + z² - x² - y²)
```

This maps each quaternion to its corresponding polarization state, with the remaining degree of freedom encoding OAM phase.

---

## 2. Physical Encoding

### 2.1 Poincaré Sphere (Polarization)

The polarization state of light is fully described by the Stokes vector on the Poincaré sphere:

```
         S₃ (Right Circular)
          │
          │    ╱ Linear +45°
          │  ╱
          │╱
    ──────┼────── S₁ (Horizontal)
         ╱│
       ╱  │
     ╱    │
   S₂     │
(Linear)  │
          │
    (Left Circular)
```

**Stokes Parameters:**
```
S₀ = |Eₓ|² + |Eᵧ|²     (Total intensity)
S₁ = |Eₓ|² - |Eᵧ|²     (Horizontal - Vertical)
S₂ = 2·Re(Eₓ·Eᵧ*)      (Diagonal - Antidiagonal)
S₃ = 2·Im(Eₓ·Eᵧ*)      (Right - Left circular)
```

### 2.2 Orbital Angular Momentum (OAM)

OAM modes have helical phase fronts:

```
E(r, φ, z) = A(r) · exp(iℓφ) · exp(ikz)

where:
  ℓ = topological charge (integer)
  φ = azimuthal angle
  k = wave vector
```

**Intensity Profile (Laguerre-Gaussian modes):**
```
        ℓ=0          ℓ=1          ℓ=2          ℓ=3
       ┌───┐        ┌───┐        ┌───┐        ┌───┐
       │ ● │        │ ○ │        │◯◯│        │○○○│
       │   │        │   │        │  │        │ ○ │
       └───┘        └───┘        └───┘        └───┘
      Gaussian     Donut       Double      Triple
                              Ring        Ring
```

### 2.3 Combined State Space

The full 4D state is:

```
|Ψ⟩ = |Polarization⟩ ⊗ |OAM⟩

Parameterized by quaternion:
  (w, x, y, z) ↔ (θ_pol, φ_pol, ψ_pol, ℓ_oam)
```

---

## 3. Hash-to-Quaternion Mapping

### 3.1 SHA-256 to Unit Quaternion

```python
def hash_to_quaternion(data: bytes) -> Quaternion:
    """
    Convert SHA-256 hash to unit quaternion.

    Process:
    1. Hash data to 256 bits
    2. Split into four 64-bit floats
    3. Normalize to unit quaternion
    """
    h = sha256(data).digest()  # 32 bytes

    # Extract four components (using first 16 bytes)
    w = struct.unpack('f', h[0:4])[0]
    x = struct.unpack('f', h[4:8])[0]
    y = struct.unpack('f', h[8:12])[0]
    z = struct.unpack('f', h[12:16])[0]

    # Normalize to unit quaternion
    norm = sqrt(w*w + x*x + y*y + z*z)
    return Quaternion(w/norm, x/norm, y/norm, z/norm)
```

### 3.2 Chain Rotation Accumulation

```python
def compute_lattice_rotation(trace_events: List[TraceEvent]) -> Quaternion:
    """
    Compute cumulative lattice rotation from TRACE history.

    Each event's hash rotates the lattice further.
    """
    rotation = Quaternion(1, 0, 0, 0)  # Identity

    for event in trace_events:
        event_hash = sha256(event.serialize()).digest()
        delta_rotation = hash_to_quaternion(event_hash)
        rotation = rotation * delta_rotation  # Quaternion multiply
        rotation = rotation.normalize()

    return rotation
```

---

## 4. Modulation Process

### 4.1 Encoder Pipeline

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   7 bits    │────▶│   Vertex    │────▶│  Quaternion │
│   (data)    │     │   Index     │     │    q_data   │
└─────────────┘     └─────────────┘     └─────────────┘
                                              │
┌─────────────┐     ┌─────────────┐           │
│   TRACE     │────▶│  Rotation   │           │
│   Chain     │     │   q_rot     │           │
└─────────────┘     └─────────────┘           │
                          │                    │
                          ▼                    ▼
                    ┌─────────────────────────────┐
                    │  q_transmit = q_rot ⊗ q_data │
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────┴──────────────┐
                    │         Decompose           │
                    ├──────────────┬──────────────┤
                    ▼              ▼              ▼
             ┌──────────┐   ┌──────────┐   ┌──────────┐
             │ Stokes   │   │   OAM    │   │ Optical  │
             │ (S1,S2,S3)│   │  Mode ℓ  │   │  Output  │
             └──────────┘   └──────────┘   └──────────┘
```

### 4.2 Quaternion to Stokes/OAM Decomposition

```rust
fn quaternion_to_optical(q: Quaternion) -> OpticalState {
    // Stokes parameters from quaternion
    let s1 = 2.0 * (q.x * q.y + q.w * q.z);
    let s2 = 2.0 * (q.y * q.z - q.w * q.x);
    let s3 = q.w * q.w + q.z * q.z - q.x * q.x - q.y * q.y;

    // OAM mode from scalar part phase
    let phase = (q.z).atan2(q.w);
    let oam_mode = (phase * MAX_OAM_MODES as f64 / PI).round() as i32;

    OpticalState {
        stokes: StokesVector { s1, s2, s3 },
        oam: oam_mode,
    }
}
```

### 4.3 Hardware Control

```rust
fn set_slm_state(state: OpticalState, slm: &mut SpatialLightModulator) {
    // Set polarization via liquid crystal
    let (theta, phi) = stokes_to_angles(state.stokes);
    slm.set_polarization_retardance(theta);
    slm.set_polarization_rotation(phi);

    // Set OAM via spiral phase plate
    slm.set_oam_mode(state.oam);

    // Trigger transmission
    slm.fire();
}
```

---

## 5. Demodulation Process

### 5.1 Measurement

```
┌─────────────────────────────────────────────────────────┐
│                 COHERENT RECEIVER                        │
├─────────────────────────────────────────────────────────┤
│                                                          │
│   [Optical Input]                                        │
│         │                                                │
│         ▼                                                │
│   ┌───────────────┐                                     │
│   │  Beam         │                                     │
│   │  Splitter     │                                     │
│   └───┬───────┬───┘                                     │
│       │       │                                          │
│       ▼       ▼                                          │
│  ┌────────┐ ┌────────────┐                              │
│  │ Stokes │ │ OAM Sorter │                              │
│  │Analyzer│ │            │                              │
│  └───┬────┘ └─────┬──────┘                              │
│      │            │                                      │
│      ▼            ▼                                      │
│  ┌────────┐  ┌────────┐                                 │
│  │S1,S2,S3│  │ℓ value │                                 │
│  │measured│  │detected│                                 │
│  └───┬────┘  └───┬────┘                                 │
│      │           │                                       │
│      └─────┬─────┘                                       │
│            ▼                                             │
│   ┌────────────────┐                                    │
│   │ Reconstruct    │                                    │
│   │ q_measured     │                                    │
│   └────────────────┘                                    │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### 5.2 Quaternion Reconstruction

```rust
fn optical_to_quaternion(state: MeasuredOpticalState) -> Quaternion {
    let StokesVector { s1, s2, s3 } = state.stokes;

    // Compute polarization angles
    let r = (s1*s1 + s2*s2 + s3*s3).sqrt();
    let phi = s2.atan2(s1);
    let psi = (s3 / r).acos();

    // OAM phase
    let theta = PI * state.oam as f64 / MAX_OAM_MODES as f64;

    // Reconstruct quaternion
    Quaternion {
        w: (theta / 2.0).cos() * (psi / 2.0).cos(),
        x: (theta / 2.0).sin() * (psi / 2.0).sin() * phi.cos(),
        y: (theta / 2.0).sin() * (psi / 2.0).sin() * phi.sin(),
        z: (theta / 2.0).cos() * (psi / 2.0).sin(),
    }.normalize()
}
```

### 5.3 De-rotation

```rust
fn derotate(q_measured: Quaternion, q_rotation: Quaternion) -> Quaternion {
    // Apply inverse rotation: q_data = q_rot* ⊗ q_measured ⊗ q_rot
    let q_rot_conj = q_rotation.conjugate();
    q_rot_conj * q_measured * q_rotation
}
```

---

## 6. Error Analysis

### 6.1 Noise Model

Channel noise affects quaternion components:

```
q_received = q_transmitted + n

where n ~ N(0, σ²I₄) (isotropic Gaussian noise in 4D)
```

### 6.2 Symbol Error Probability

For 600-cell constellation with minimum angular distance d_min = π/5:

```
P_symbol ≈ 120 · Q(d_min · √(SNR/2))

where Q(x) = ½·erfc(x/√2) is the Q-function
```

### 6.3 Geometric Margin

The Voronoi region around each vertex provides:
- Angular radius: θ_vor = π/10 ≈ 18°
- Solid angle: Ω = 4π/120 ≈ 0.105 sr
- Maximum tolerable angular error: ±18° in any direction

---

## 7. Implementation Constants

```rust
pub mod constants {
    /// Maximum OAM modes supported
    pub const MAX_OAM_MODES: i32 = 16;

    /// Golden ratio (φ)
    pub const PHI: f64 = 1.6180339887498948482;

    /// Minimum vertex angular distance (radians)
    pub const MIN_VERTEX_DISTANCE: f64 = std::f64::consts::PI / 5.0;

    /// Voronoi region angular radius
    pub const VORONOI_RADIUS: f64 = std::f64::consts::PI / 10.0;

    /// Bits per symbol (log2(120))
    pub const BITS_PER_SYMBOL: u32 = 7;

    /// C-band center wavelength (nm)
    pub const WAVELENGTH_NM: f64 = 1550.0;
}
```

---

## 8. SLERP for Continuous Rotation

For smooth lattice transitions (if needed):

```rust
/// Spherical Linear Interpolation between quaternions
fn slerp(q0: Quaternion, q1: Quaternion, t: f64) -> Quaternion {
    let mut dot = q0.dot(&q1);

    // Ensure shortest path
    let q1 = if dot < 0.0 {
        dot = -dot;
        -q1
    } else {
        q1
    };

    if dot > 0.9995 {
        // Linear interpolation for very close quaternions
        let result = q0 + (q1 - q0) * t;
        return result.normalize();
    }

    let theta_0 = dot.acos();
    let theta = theta_0 * t;

    let q2 = (q1 - q0 * dot).normalize();

    q0 * theta.cos() + q2 * theta.sin()
}
```

---

## 9. Test Vectors

### 9.1 Identity State

```
Input:  Data bits = 0000000 (vertex 0)
        No rotation (identity lattice)

Output: q = (1.0, 0.0, 0.0, 0.0)
        Stokes = (0, 0, 1)  [Horizontal linear]
        OAM = 0
```

### 9.2 Rotated State

```
Input:  Data bits = 0000001 (vertex 1)
        Rotation = (0.5, 0.5, 0.5, 0.5)

Output: q = rotation ⊗ vertex[1]
        Verify: |q| = 1.0
        Verify: Stokes² ≤ 1.0
        Verify: |OAM| ≤ MAX_OAM_MODES
```

### 9.3 Hash Chain Consistency

```
Events: [E1, E2, E3]
H1 = SHA256(E1)
H2 = SHA256(E2)
H3 = SHA256(E3)

R1 = hash_to_quat(H1)
R2 = hash_to_quat(H2)
R3 = hash_to_quat(H3)

Final rotation: R = R1 ⊗ R2 ⊗ R3

Verify: R is unit quaternion
Verify: Different event order → different R
```

---

*End of Spinor Modulation Specification*
