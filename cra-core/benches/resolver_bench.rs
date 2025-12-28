//! Benchmarks for the CRA Resolver

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;

use cra_core::{AtlasManifest, CARPRequest, Resolver};

fn create_test_atlas() -> AtlasManifest {
    serde_json::from_value(json!({
        "atlas_version": "1.0",
        "atlas_id": "com.bench.test",
        "version": "1.0.0",
        "name": "Benchmark Atlas",
        "description": "Atlas for benchmarking",
        "domains": ["test"],
        "capabilities": [],
        "policies": [
            {
                "policy_id": "deny-delete",
                "type": "deny",
                "actions": ["*.delete"],
                "reason": "No deletes"
            }
        ],
        "actions": [
            {
                "action_id": "test.get",
                "name": "Get",
                "description": "Get resource",
                "parameters_schema": { "type": "object" },
                "risk_tier": "low"
            },
            {
                "action_id": "test.list",
                "name": "List",
                "description": "List resources",
                "parameters_schema": { "type": "object" },
                "risk_tier": "low"
            },
            {
                "action_id": "test.create",
                "name": "Create",
                "description": "Create resource",
                "parameters_schema": { "type": "object" },
                "risk_tier": "medium"
            },
            {
                "action_id": "test.update",
                "name": "Update",
                "description": "Update resource",
                "parameters_schema": { "type": "object" },
                "risk_tier": "medium"
            },
            {
                "action_id": "test.delete",
                "name": "Delete",
                "description": "Delete resource",
                "parameters_schema": { "type": "object" },
                "risk_tier": "high"
            }
        ]
    }))
    .unwrap()
}

fn bench_resolver_creation(c: &mut Criterion) {
    c.bench_function("resolver_new", |b| {
        b.iter(|| {
            let resolver = Resolver::new();
            black_box(resolver)
        })
    });
}

fn bench_atlas_load(c: &mut Criterion) {
    c.bench_function("atlas_load", |b| {
        b.iter(|| {
            let mut resolver = Resolver::new();
            let atlas = create_test_atlas();
            resolver.load_atlas(atlas).unwrap();
            black_box(resolver)
        })
    });
}

fn bench_session_create(c: &mut Criterion) {
    let mut resolver = Resolver::new();
    resolver.load_atlas(create_test_atlas()).unwrap();

    c.bench_function("session_create", |b| {
        b.iter(|| {
            let session_id = resolver.create_session("bench-agent", "Benchmark goal").unwrap();
            black_box(session_id)
        })
    });
}

fn bench_resolve(c: &mut Criterion) {
    let mut resolver = Resolver::new();
    resolver.load_atlas(create_test_atlas()).unwrap();
    let session_id = resolver.create_session("bench-agent", "Benchmark goal").unwrap();

    c.bench_function("resolve", |b| {
        b.iter(|| {
            let request = CARPRequest::new(
                session_id.clone(),
                "bench-agent".to_string(),
                "I want to manage resources".to_string(),
            );
            let resolution = resolver.resolve(&request).unwrap();
            black_box(resolution)
        })
    });
}

fn bench_execute(c: &mut Criterion) {
    let mut resolver = Resolver::new();
    resolver.load_atlas(create_test_atlas()).unwrap();
    let session_id = resolver.create_session("bench-agent", "Benchmark goal").unwrap();

    c.bench_function("execute", |b| {
        b.iter(|| {
            let result = resolver.execute(
                &session_id,
                "res-1",
                "test.get",
                json!({"id": "123"}),
            ).unwrap();
            black_box(result)
        })
    });
}

fn bench_verify_chain(c: &mut Criterion) {
    let mut resolver = Resolver::new();
    resolver.load_atlas(create_test_atlas()).unwrap();
    let session_id = resolver.create_session("bench-agent", "Benchmark goal").unwrap();

    // Generate some events
    for _ in 0..100 {
        let request = CARPRequest::new(
            session_id.clone(),
            "bench-agent".to_string(),
            "Test".to_string(),
        );
        let _ = resolver.resolve(&request);
    }

    c.bench_function("verify_chain_100_events", |b| {
        b.iter(|| {
            let result = resolver.verify_chain(&session_id).unwrap();
            black_box(result)
        })
    });
}

criterion_group!(
    benches,
    bench_resolver_creation,
    bench_atlas_load,
    bench_session_create,
    bench_resolve,
    bench_execute,
    bench_verify_chain
);

criterion_main!(benches);
