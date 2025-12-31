//! Benchmark for Voronoi lookup performance.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cspm_core::{
    polytope::{Hexacosichoron, VoronoiLookup},
    quaternion::Quaternion,
};

fn bench_voronoi_lookup(c: &mut Criterion) {
    let hexacosichoron = Hexacosichoron::new();
    let voronoi = VoronoiLookup::new(&hexacosichoron);

    // Generate random test quaternions
    let test_points: Vec<Quaternion> = (0..1000)
        .map(|i| {
            Quaternion::new(
                (i as f64 * 0.001).sin(),
                (i as f64 * 0.002).cos(),
                (i as f64 * 0.003).sin(),
                (i as f64 * 0.004).cos(),
            ).normalize()
        })
        .collect();

    c.bench_function("voronoi_single_lookup", |b| {
        let q = test_points[0];
        b.iter(|| {
            voronoi.nearest(black_box(&q))
        })
    });

    c.bench_function("voronoi_1000_lookups", |b| {
        b.iter(|| {
            for q in &test_points {
                black_box(voronoi.nearest(q));
            }
        })
    });

    c.bench_function("hexacosichoron_direct_search", |b| {
        let q = test_points[0];
        b.iter(|| {
            hexacosichoron.nearest_vertex(black_box(&q))
        })
    });
}

fn bench_encoding(c: &mut Criterion) {
    use cspm_core::{CspmEncoder, GenesisConfig};

    let config = GenesisConfig::new(b"benchmark_secret");

    c.bench_function("encode_symbol", |b| {
        let mut encoder = CspmEncoder::new(config.clone());
        b.iter(|| {
            encoder.encode_symbol(black_box(42)).unwrap()
        })
    });

    c.bench_function("encode_1000_symbols", |b| {
        b.iter(|| {
            let mut encoder = CspmEncoder::new(config.clone());
            for i in 0..1000u8 {
                black_box(encoder.encode_symbol(i % 120).unwrap());
            }
        })
    });
}

fn bench_decoding(c: &mut Criterion) {
    use cspm_core::{CspmEncoder, CspmDecoder, GenesisConfig};

    let config = GenesisConfig::new(b"benchmark_secret");
    let mut encoder = CspmEncoder::new(config.clone());

    // Pre-encode symbols
    let symbols: Vec<_> = (0..1000u8)
        .map(|i| encoder.encode_symbol(i % 120).unwrap())
        .collect();

    let quaternions: Vec<_> = symbols.iter().map(|s| s.quaternion).collect();

    c.bench_function("decode_1000_symbols", |b| {
        b.iter(|| {
            let mut decoder = CspmDecoder::new(config.clone());
            for q in &quaternions {
                black_box(decoder.decode_quaternion(q).unwrap());
            }
        })
    });
}

criterion_group!(benches, bench_voronoi_lookup, bench_encoding, bench_decoding);
criterion_main!(benches);
