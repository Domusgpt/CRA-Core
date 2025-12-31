# CSPM Development Roadmap

This document outlines the development phases for bringing CSPM from simulation to production-ready optical hardware.

---

## Current Status: Phase 2 In Progress

### Phase 1: Core Library & Simulation (COMPLETE)

| Component | Status | Notes |
|-----------|--------|-------|
| Quaternion mathematics | ✅ Complete | Hamilton product, SLERP, axis-angle |
| 600-cell geometry | ✅ Complete | All 120 vertices, adjacency matrix |
| Gray code mapping | ✅ Complete | Hamming-optimized bit assignment |
| Voronoi tessellation | ✅ Complete | O(1) nearest vertex lookup |
| Hash chain | ✅ Complete | SHA-256 based lattice rotation |
| Encoder/Decoder | ✅ Complete | Full encode/decode pipeline |
| Fiber channel model | ✅ Complete | PMD, dispersion, ASE, Kerr |
| FSO channel model | ✅ Complete | Kolmogorov turbulence |
| Hardware models | ✅ Complete | SLM, detector, ADC |
| Monte Carlo BER | ✅ Complete | Statistical validation |
| Baseline comparison | ✅ Complete | 64-QAM + LDPC |
| Validation report | ✅ Complete | Patent reduction to practice |

---

## Phase 2: Production Readiness

### 2.1 Channel Equalization ✅ COMPLETE

| Feature | Status | Location |
|---------|--------|----------|
| Pilot symbol insertion | ✅ Complete | `equalization/pilot.rs` |
| Least-squares estimator | ✅ Complete | `equalization/estimator.rs` |
| CMA blind equalizer | ✅ Complete | `equalization/cma.rs` |
| Decision-directed updates | ✅ Complete | `equalization/adaptive.rs` |
| Adaptive mode transitions | ✅ Complete | Acquisition → Tracking → DD |
| Frame-based equalizer | ✅ Complete | Batch processing support |

**Delivered:** `cspm-core/src/equalization/` module with 19 new tests

**Key Capabilities:**
- Pilot patterns with configurable spacing (default: 16 symbols, 6.25% overhead)
- Weighted least-squares channel estimation with exponential forgetting
- CMA blind equalization exploiting unit quaternion property
- Channel interpolation for smooth tracking between pilots
- Automatic mode transitions based on acquisition state

**96 tests passing total.**

### 2.2 Synchronization Protocol (Priority: HIGH)

| Feature | Description | Effort |
|---------|-------------|--------|
| Preamble design | Unique word for frame sync | Low |
| Sequence number encoding | Embed seq# in symbol stream | Low |
| Hash chain checkpoint | Periodic state snapshots | Medium |
| Resync protocol | Recover from packet loss | Medium |
| Clock recovery | Symbol timing estimation | High |

**Deliverable:** `cspm-core/src/sync/` module with framing and recovery

### 2.3 Performance Optimization (Priority: MEDIUM)

| Feature | Description | Effort |
|---------|-------------|--------|
| SIMD Voronoi | AVX2/NEON accelerated lookup | Medium |
| Batch encoding | Process multiple symbols in parallel | Low |
| LUT-based hash | Precompute rotation tables | Medium |
| Zero-copy buffers | Avoid allocations in hot path | Medium |
| GPU acceleration | CUDA/Metal for high-throughput | High |

**Deliverable:** `cargo bench` showing <100ns per symbol

---

## Phase 3: Hardware Interface

### 3.1 SLM Controller Interface (Priority: HIGH)

| Feature | Description | Effort |
|---------|-------------|--------|
| Phase pattern generation | Convert quaternion to SLM hologram | High |
| OAM mode synthesis | Generate spiral phase patterns | High |
| Refresh rate handling | 60-200 Hz frame buffering | Medium |
| Calibration routines | Phase response linearization | High |

**Target Hardware:**
- Hamamatsu X13138-02 (LCOS-SLM)
- Holoeye PLUTO-2.1
- Meadowlark P1920

**Deliverable:** `cspm-hardware/slm/` crate with device drivers

### 3.2 Coherent Receiver Interface (Priority: HIGH)

| Feature | Description | Effort |
|---------|-------------|--------|
| Stokes parameter extraction | Polarimeter measurements | Medium |
| OAM mode detection | Mode sorter readout | High |
| ADC streaming | High-speed sample acquisition | Medium |
| Real-time decoding | FPGA offload for low latency | Very High |

**Target Hardware:**
- Thorlabs PAX1000 (Polarimeter)
- Custom OAM mode sorter
- Teledyne ADQ7DC (14-bit, 10 GSPS)

**Deliverable:** `cspm-hardware/receiver/` crate with device drivers

### 3.3 FPGA Accelerator (Priority: MEDIUM)

| Feature | Description | Effort |
|---------|-------------|--------|
| Voronoi lookup | <10 cycle nearest vertex | High |
| Hash chain | Pipelined SHA-256 | Medium |
| Quaternion multiply | Fixed-point Hamilton product | Medium |
| DMA interface | PCIe/AXI streaming | High |

**Target Platforms:**
- Xilinx Alveo U250 (Data center)
- AMD Kria KV260 (Edge)
- Intel Agilex (Low latency)

**Deliverable:** `cspm-fpga/` repository with RTL and drivers

---

## Phase 4: Integration & Testing

### 4.1 Lab Demonstration (Priority: HIGH)

| Milestone | Description |
|-----------|-------------|
| Back-to-back | SLM → Free-space → Receiver (1m) |
| Fiber loopback | 10km SMF-28 with EDFA |
| OAM transmission | Demonstrate mode stability |
| BER measurement | Validate <10⁻⁹ at target SNR |
| Latency profiling | End-to-end <1μs target |

### 4.2 Environmental Testing

| Test | Specification |
|------|---------------|
| Temperature | -5°C to +45°C operation |
| Vibration | IEC 60068-2-6 compliance |
| Humidity | 10-90% non-condensing |
| EMI | FCC Part 15 Class B |

### 4.3 Standards Compliance

| Standard | Application |
|----------|-------------|
| IEEE 802.3 | Ethernet framing adaptation |
| ITU-T G.709 | OTN wrapper compatibility |
| FIPS 140-3 | Cryptographic module validation |

---

## Phase 5: Production Deployment

### 5.1 Data Center Interconnect

| Feature | Target |
|---------|--------|
| Link distance | 2-10 km |
| Data rate | 400 Gbps aggregate |
| Latency | <500 ns |
| Availability | 99.999% |

### 5.2 Long-Haul Fiber

| Feature | Target |
|---------|--------|
| Link distance | 500-2000 km |
| Regeneration | Every 80-100 km |
| Data rate | 100 Gbps per wavelength |
| OSNR margin | 2 dB improvement over QAM |

### 5.3 Free-Space Optical

| Feature | Target |
|---------|--------|
| Link distance | 1-10 km terrestrial |
| Turbulence tolerance | Cn² up to 10⁻¹³ |
| Tracking | Sub-μrad pointing accuracy |
| Availability | 99.9% (weather dependent) |

---

## Development Priorities

### Immediate (Next Sprint)

1. **Channel equalization module** - Required for real-world BER performance
2. **Pilot symbol design** - Enable channel estimation
3. **SIMD optimization** - Reduce per-symbol latency

### Short-Term (1-2 Months)

4. Synchronization protocol with resync capability
5. SLM hologram generation algorithm
6. Coherent receiver Stokes extraction

### Medium-Term (3-6 Months)

7. FPGA prototype for Voronoi lookup
8. Lab demonstration setup
9. Patent filing completion

### Long-Term (6-12 Months)

10. Environmental qualification
11. Standards body engagement (IEEE, ITU-T)
12. Partner integration (equipment vendors)

---

## Technical Debt & Improvements

### Code Quality

| Item | Priority | Description |
|------|----------|-------------|
| Unused imports cleanup | Low | Remove compiler warnings |
| Documentation | Medium | Expand rustdoc comments |
| Error handling | Medium | Replace unwrap() with proper errors |
| Logging | Medium | Add tracing instrumentation |

### Testing

| Item | Priority | Description |
|------|----------|-------------|
| Property tests | Medium | Proptest for encoder/decoder |
| Fuzzing | Medium | AFL/libFuzzer for robustness |
| Benchmark suite | High | Criterion.rs for performance |
| Integration tests | High | Full pipeline validation |

### Architecture

| Item | Priority | Description |
|------|----------|-------------|
| Async support | Medium | Tokio integration for I/O |
| No-std support | Low | Embedded deployment option |
| C FFI | Medium | Bindings for other languages |
| Python bindings | Medium | PyO3 wrapper for research |

---

## Resource Requirements

### Personnel

| Role | FTE | Responsibility |
|------|-----|----------------|
| DSP Engineer | 1.0 | Equalization, sync, FPGA |
| Photonics Engineer | 0.5 | SLM, receiver integration |
| Software Engineer | 1.0 | Rust library, drivers |
| Test Engineer | 0.5 | Lab setup, BER measurement |

### Equipment (Estimated)

| Item | Cost | Purpose |
|------|------|---------|
| LCOS-SLM | $15-30k | Phase modulation |
| Coherent receiver | $20-50k | Detection system |
| Polarimeter | $5-10k | Stokes measurement |
| Fiber spools | $1-5k | Channel testing |
| FPGA dev kit | $5-15k | Acceleration prototype |

---

## Success Metrics

| Phase | Metric | Target |
|-------|--------|--------|
| 2 | Equalized BER in fiber | <10⁻⁶ at 20 dB OSNR |
| 3 | Hardware encode latency | <1 μs |
| 4 | Lab demo BER | <10⁻⁹ |
| 5 | Production link uptime | 99.999% |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

For architecture details, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

---

*Last updated: 2025*
