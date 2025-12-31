# CSPM Hardware Acquisition Plan

## Overview

This document outlines the hardware needed to demonstrate CSPM (Cryptographically-Seeded Polytopal Modulation) in a lab setting. The system encodes data onto 4D quaternion states mapped to optical polarization + OAM modes.

## Minimum Viable Lab Demo (Budget Tier)

### Tier 1: Basic Proof-of-Concept (~$5K-15K)

| Component | Purpose | Options | Est. Cost |
|-----------|---------|---------|-----------|
| **SLM** | Generate OAM modes + polarization | Holoeye LC2012 (used), Thorlabs SLM (~$3K refurb) | $2K-5K |
| **Laser** | Coherent source | 1550nm DFB laser or HeNe 633nm | $500-2K |
| **Polarization Optics** | Generate/analyze states | Half-wave plates, QWPs, PBS | $500-1K |
| **Camera/Detector** | Capture interference patterns | USB camera or photodiode array | $200-1K |
| **Optical Table** | Stability | Small breadboard 2'x3' | $500-1K |
| **Mounts/Optics** | Beam routing | Mirrors, lenses, mounts | $500-1K |

**Capabilities:**
- Demonstrate OAM mode generation
- Show polarization state control
- Validate constellation geometry
- Measure BER for single channel

### Tier 2: Full Demonstration (~$30K-80K)

| Component | Purpose | Options | Est. Cost |
|-----------|---------|---------|-----------|
| **High-res SLM** | Full constellation | Hamamatsu X13138, Meadowlark | $15K-30K |
| **Coherent Receiver** | Phase/amplitude recovery | Thorlabs coherent Rx, custom | $10K-20K |
| **Polarimeter** | Full Stokes measurement | Thorlabs PAX1000 | $8K-15K |
| **FPGA/DAQ** | Real-time processing | Xilinx Zynq, Red Pitaya | $2K-5K |
| **Optical Fiber** | Channel emulation | SMF-28, turbulence emulator | $1K-5K |

**Capabilities:**
- Full 120-vertex constellation
- Real-time encode/decode
- Channel characterization
- Cryptographic key exchange demo

## Component Deep Dive

### Spatial Light Modulators (SLM)

The SLM is the core transmitter component. Options:

1. **Holoeye LC2012** (~$3K used)
   - 1024x768 pixels
   - 8-bit phase modulation
   - Good for initial prototyping

2. **Hamamatsu LCOS-SLM X13138** (~$25K)
   - 1920x1080 pixels
   - High refresh rate
   - Production-quality

3. **Meadowlark P1920** (~$20K)
   - 1920x1152 pixels
   - Very high damage threshold
   - Scientific applications

**Integration:** The `SlmController` trait in `hardware/slm.rs` abstracts these devices. The `HologramGenerator` computes phase patterns for each.

### Coherent Receivers

To decode CSPM, you need:

1. **4-Detector Polarimeter** (~$5K)
   - Extracts Stokes parameters S1, S2, S3
   - Uses PBS + waveplates + 4 photodiodes
   - DIY-friendly

2. **Thorlabs PAX1000** (~$12K)
   - Complete Stokes polarimeter
   - USB interface
   - Sub-degree accuracy

3. **Commercial Coherent Rx** (~$30K+)
   - Full I/Q + polarization diversity
   - Used in telecom research
   - OAM detection requires additional optics

**Integration:** The `CoherentReceiver` trait in `hardware/receiver.rs` with `StokesExtractor` for detector processing.

### OAM Mode Sorters

To distinguish OAM modes:

1. **Fork Grating + Camera** (~$500)
   - Simple interferometric detection
   - Works for ±3 modes
   - Good for demos

2. **Log-Polar Transform Optics** (~$5K)
   - Separate OAM modes spatially
   - Requires custom optics
   - High mode count

3. **Commercial OAM Sorter** (~$15K+)
   - Cailabs PROTEUS
   - Up to 10 modes
   - Fiber-coupled

## Acquisition Strategy

### Phase 1: Minimal Demo (Month 1-2)

**Goal:** Prove OAM + polarization modulation works

**Equipment:**
- Used SLM (eBay, university surplus)
- HeNe laser
- Basic polarization kit
- USB camera
- Breadboard

**Budget:** ~$5K

**Demonstration:**
- Generate visible OAM beams (donut modes)
- Encode/decode a few symbols
- Show vertex-snapping correction

### Phase 2: Multi-symbol Demo (Month 3-4)

**Goal:** Demonstrate constellation navigation

**Equipment:**
- Add polarimeter or build 4-detector system
- Add FPGA for real-time processing
- Fiber test channel

**Budget:** +$10K (total ~$15K)

**Demonstration:**
- Encode arbitrary data
- Measure BER vs SNR
- Show cryptographic key exchange

### Phase 3: Full System (Month 5+)

**Goal:** Production-quality demo

**Equipment:**
- High-res SLM
- Commercial coherent receiver
- Full lab setup

**Budget:** +$30K (total ~$50K)

**Demonstration:**
- Full 120-vertex constellation
- Real-time Gbps throughput
- Conference/publication-ready

## Alternative Approaches

### Software-Only Simulation

If hardware is not immediately available:

1. **GPU-Accelerated Simulation**
   - The `simulation/` module models all channel effects
   - Realistic BER curves without hardware
   - Good for algorithm development

2. **Hardware-in-the-Loop**
   - Connect to simulated hardware via USB/TCP
   - Test control interfaces
   - Gradual hardware integration

### Collaborative Access

1. **University Labs**
   - Many universities have SLMs for teaching
   - Optical communications labs often have coherent Rx
   - Approach physics/EE departments

2. **Maker Spaces**
   - Some have optical equipment
   - Lower cost entry point

3. **Equipment Rental**
   - Companies rent lab equipment
   - Useful for short-term demos

## Recommended First Purchase

For immediate progress toward lab demo:

**Priority 1: SLM** ($2K-5K)
- Look for used Holoeye LC2012 or similar
- eBay, university surplus, laser forums
- This enables OAM generation immediately

**Priority 2: Laser + Optics** ($1K-2K)
- HeNe 633nm is cheapest/easiest
- 1550nm DFB for telecom compatibility
- Basic polarization optics kit

**Priority 3: Detection** ($500-2K)
- Start with USB camera for OAM patterns
- Build 4-detector polarimeter DIY
- Upgrade to commercial later

## Integration with Codebase

The hardware module supports:

```rust
// Create simulated or real hardware
let mut slm = SlmController::new(SlmConfig {
    device_type: SlmDeviceType::Simulated, // or Hamamatsu, Holoeye, etc.
    resolution: (1920, 1080),
    wavelength_nm: 1550.0,
    ..Default::default()
});

let mut rx = CoherentReceiver::new(ReceiverConfig {
    device_type: ReceiverDeviceType::Simulated, // or Polarimeter4, CoherentRx
    ..Default::default()
});

// Connect and use
slm.connect()?;
slm.set_state(&optical_state)?;

rx.connect()?;
let measured = rx.measure()?;
```

When real hardware arrives, just change the `device_type` and add hardware-specific drivers.

## Next Steps

1. **Survey available funds** - What's your budget range?
2. **Check university surplus** - Often has SLMs, lasers, optics
3. **Join photonics forums** - PhotonLexicon, Reddit r/lasers for used gear
4. **Contact vendors** - Request academic discounts
5. **Start with simulation** - Continue algorithm development in parallel

Let me know your budget constraints and I can refine recommendations.
