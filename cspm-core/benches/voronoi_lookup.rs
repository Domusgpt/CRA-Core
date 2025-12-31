//! Benchmark for Voronoi lookup and CSPM performance.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use cspm_core::{
    polytope::{Hexacosichoron, VoronoiLookup},
    quaternion::Quaternion,
    performance::{SimdVoronoi, BatchEncoder, BatchDecoder},
};

fn bench_voronoi_lookup(c: &mut Criterion) {
    let hexacosichoron = Hexacosichoron::new();
    let voronoi = VoronoiLookup::new(&hexacosichoron);
    let simd_voronoi = SimdVoronoi::new(&hexacosichoron);

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

    c.bench_function("simd_voronoi_single_lookup", |b| {
        let q = test_points[0];
        b.iter(|| {
            simd_voronoi.nearest(black_box(&q))
        })
    });

    c.bench_function("voronoi_1000_lookups", |b| {
        b.iter(|| {
            for q in &test_points {
                black_box(voronoi.nearest(q));
            }
        })
    });

    c.bench_function("simd_voronoi_1000_lookups", |b| {
        b.iter(|| {
            for q in &test_points {
                black_box(simd_voronoi.nearest(q));
            }
        })
    });

    c.bench_function("simd_voronoi_batch_1000", |b| {
        b.iter(|| {
            black_box(simd_voronoi.nearest_batch(&test_points))
        })
    });

    c.bench_function("hexacosichoron_direct_search", |b| {
        let q = test_points[0];
        b.iter(|| {
            hexacosichoron.nearest_vertex(black_box(&q))
        })
    });
}

fn bench_simd_nearest_4(c: &mut Criterion) {
    let hexacosichoron = Hexacosichoron::new();
    let simd_voronoi = SimdVoronoi::new(&hexacosichoron);

    let queries: [Quaternion; 4] = [
        hexacosichoron.vertices()[10].q,
        hexacosichoron.vertices()[20].q,
        hexacosichoron.vertices()[30].q,
        hexacosichoron.vertices()[40].q,
    ];

    c.bench_function("simd_nearest_4", |b| {
        b.iter(|| {
            simd_voronoi.nearest_4(black_box(&queries))
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

fn bench_batch_processing(c: &mut Criterion) {
    let mut encoder = BatchEncoder::default();
    let mut decoder = BatchDecoder::default();

    let symbols: Vec<u8> = (0..1000).map(|i| (i % 120) as u8).collect();

    c.bench_function("batch_encode_1000", |b| {
        b.iter(|| {
            black_box(encoder.encode_batch(&symbols))
        })
    });

    // Pre-encode for decode benchmark
    let encoded = encoder.encode_batch(&symbols);

    c.bench_function("batch_decode_1000", |b| {
        b.iter(|| {
            black_box(decoder.decode_batch(&encoded))
        })
    });

    // Bytes throughput test
    let data = vec![0u8; 1000];

    c.bench_function("batch_encode_1000_bytes", |b| {
        b.iter(|| {
            black_box(encoder.encode_bytes(&data))
        })
    });

    let encoded_bytes = encoder.encode_bytes(&data);

    c.bench_function("batch_decode_to_bytes", |b| {
        b.iter(|| {
            black_box(decoder.decode_to_bytes(&encoded_bytes))
        })
    });
}

fn bench_throughput(c: &mut Criterion) {
    use criterion::Throughput;

    let mut group = c.benchmark_group("throughput");

    for size in [100, 1000, 10000].iter() {
        let mut encoder = BatchEncoder::default();
        let symbols: Vec<u8> = (0..*size).map(|i| (i % 120) as u8).collect();

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("encode", size),
            &symbols,
            |b, symbols| {
                b.iter(|| encoder.encode_batch(symbols))
            }
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_voronoi_lookup,
    bench_simd_nearest_4,
    bench_encoding,
    bench_decoding,
    bench_batch_processing,
    bench_throughput
);
criterion_main!(benches);
