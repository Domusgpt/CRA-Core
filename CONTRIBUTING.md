# Contributing to CSPM

Thank you for your interest in contributing to CSPM! This document provides guidelines for development.

---

## Getting Started

### Prerequisites

- Rust 1.70+ (stable)
- Git
- Basic understanding of optical communications or quaternion mathematics

### Setup

```bash
# Clone the repository
git clone https://github.com/Domusgpt/CRA-Core.git
cd CRA-Core/cspm-core

# Build
cargo build

# Run tests
cargo test

# Run examples
cargo run --example basic_transmission
cargo run --example channel_validation
```

---

## Development Workflow

### Branch Naming

```
feature/description    # New features
fix/issue-number       # Bug fixes
docs/topic            # Documentation
perf/component        # Performance improvements
```

### Commit Messages

Follow conventional commits:

```
feat: add channel equalization module
fix: correct PMD rotation direction
docs: update architecture diagram
perf: SIMD optimize Voronoi lookup
test: add property tests for encoder
```

### Pull Request Process

1. Create a feature branch from `main`
2. Make your changes with tests
3. Ensure `cargo test` passes
4. Ensure `cargo clippy` has no warnings
5. Update documentation if needed
6. Submit PR with clear description

---

## Code Style

### Formatting

```bash
cargo fmt --all
```

### Linting

```bash
cargo clippy -- -D warnings
```

### Documentation

- All public items must have rustdoc comments
- Include examples for non-trivial functions
- Use `///` for item docs, `//!` for module docs

```rust
/// Computes the nearest vertex in the 600-cell constellation.
///
/// # Arguments
///
/// * `q` - Query quaternion (must be normalized)
///
/// # Returns
///
/// Index of the nearest vertex (0-119)
///
/// # Example
///
/// ```
/// use cspm_core::polytope::VoronoiLookup;
/// let lookup = VoronoiLookup::new(&hexacosichoron);
/// let nearest = lookup.nearest(&received_quaternion);
/// ```
pub fn nearest(&self, q: &Quaternion) -> usize {
    // ...
}
```

---

## Testing Guidelines

### Unit Tests

Each module should have a `tests` submodule:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specific_behavior() {
        // Arrange
        let input = ...;

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

### Test Categories

1. **Correctness**: Verify mathematical properties
2. **Roundtrip**: Encode → decode should recover input
3. **Edge cases**: Empty input, maximum values, boundary conditions
4. **Invariants**: Quaternions normalized, vertices in range

### Running Specific Tests

```bash
# Run all tests
cargo test

# Run tests in specific module
cargo test quaternion

# Run single test
cargo test test_hamilton_product

# Run with output
cargo test -- --nocapture
```

---

## Module Guidelines

### Adding a New Module

1. Create `src/new_module/mod.rs`
2. Add `pub mod new_module;` to `src/lib.rs`
3. Add re-exports if public API
4. Write tests
5. Add documentation

### Module Structure

```
src/new_module/
├── mod.rs          # Module root, re-exports
├── types.rs        # Data structures
├── operations.rs   # Core logic
└── tests.rs        # Unit tests (or inline in mod.rs)
```

---

## Performance Considerations

### Hot Path Optimization

The encoding/decoding pipeline is performance-critical:

1. **Avoid allocations**: Use pre-allocated buffers
2. **Use SIMD**: For quaternion and vector operations
3. **Cache-friendly**: Sequential memory access
4. **Branch-free**: Avoid conditionals in inner loops

### Benchmarking

```bash
cargo bench
```

Add benchmarks for new performance-sensitive code:

```rust
// benches/my_benchmark.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_function(c: &mut Criterion) {
    c.bench_function("my_function", |b| {
        b.iter(|| {
            // Code to benchmark
        })
    });
}

criterion_group!(benches, benchmark_function);
criterion_main!(benches);
```

---

## Security Considerations

### Cryptographic Code

- Use constant-time operations for secret-dependent branches
- Never log or print secret material
- Clear secrets from memory when done
- Use `zeroize` crate for sensitive data

### Hash Chain

- SHA-256 is the required hash function
- Do not reduce hash output or use weak hashes
- Verify chain continuity in decoder

---

## Areas for Contribution

### High Priority

| Area | Description | Skills Needed |
|------|-------------|---------------|
| Channel equalization | Implement CMA/DD algorithms | DSP |
| Synchronization | Frame sync and resync | Protocol design |
| SIMD optimization | AVX2/NEON for quaternions | Low-level optimization |

### Medium Priority

| Area | Description | Skills Needed |
|------|-------------|---------------|
| Python bindings | PyO3 wrapper | Python, Rust FFI |
| Documentation | Expand examples | Technical writing |
| Property tests | Proptest integration | Testing |

### Low Priority

| Area | Description | Skills Needed |
|------|-------------|---------------|
| no_std support | Embedded deployment | Embedded Rust |
| Alternative polytopes | 24-cell, 120-cell | Geometry |
| Visualization | 4D projection tools | Graphics |

---

## Communication

### Issues

- Use issue templates
- Provide minimal reproduction for bugs
- Search existing issues first

### Discussions

- Use GitHub Discussions for questions
- Tag with appropriate topic

### Code Review

- Be constructive and respectful
- Focus on code, not person
- Explain reasoning for suggestions

---

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).

---

## Recognition

Contributors will be acknowledged in:
- README.md contributors section
- Release notes
- Academic publications (if applicable)

---

*Thank you for contributing to CSPM!*
