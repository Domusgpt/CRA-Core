//! Detailed Context Injection Demonstration
//!
//! This test shows EXACTLY what an agent sees with and without context injection.

use cra_core::{
    Resolver, CARPRequest,
    atlas::{AtlasManifest, AtlasContextBlock},
    trace::EventType,
};
use serde_json::json;

/// Create a minimal atlas WITHOUT context blocks
fn create_no_context_atlas() -> AtlasManifest {
    serde_json::from_value(json!({
        "atlas_version": "1.0",
        "atlas_id": "demo.no-context",
        "version": "1.0.0",
        "name": "No Context Atlas",
        "description": "Atlas with NO context blocks - agent gets no guidance",
        "domains": ["demo"],
        "capabilities": [],
        "policies": [],
        "actions": [
            {
                "action_id": "code.modify",
                "name": "Modify Code",
                "description": "Edit source code",
                "parameters_schema": {"type": "object"},
                "risk_tier": "medium"
            }
        ]
    })).unwrap()
}

/// Create an atlas WITH context blocks
fn create_with_context_atlas() -> AtlasManifest {
    let mut atlas: AtlasManifest = serde_json::from_value(json!({
        "atlas_version": "1.0",
        "atlas_id": "demo.with-context",
        "version": "1.0.0",
        "name": "Context-Enabled Atlas",
        "description": "Atlas WITH context blocks - agent gets critical guidance",
        "domains": ["demo"],
        "capabilities": [],
        "policies": [],
        "actions": [
            {
                "action_id": "code.modify",
                "name": "Modify Code",
                "description": "Edit source code",
                "parameters_schema": {"type": "object"},
                "risk_tier": "medium"
            }
        ]
    })).unwrap();

    // Add context blocks for hash-related work
    atlas.context_blocks = vec![
        AtlasContextBlock {
            context_id: "critical-hash-warning".to_string(),
            name: "Hash Computation Warning".to_string(),
            priority: 200,
            content: r#"# CRITICAL: Hash Computation Rules

**NEVER reimplement hash logic. Use TRACEEvent::compute_hash().**

## Why This Matters
Hash chain integrity is CRITICAL for CRA's cryptographic audit trail.
If you reimplement hash computation:
1. Chain verification will FAIL
2. Audit trail becomes invalid
3. The system loses its security guarantees

## Correct Pattern
```rust
// CORRECT: Use existing implementation
let hash = event.compute_hash();
```

## Wrong Pattern (NEVER DO THIS)
```rust
// WRONG: Reimplementing hash logic
let mut hasher = Sha256::new();
hasher.update(serde_json::to_string(&event)?); // BREAKS CHAIN!
```

## Key Insight
Use `canonical_json()` not `serde_json::to_string()` - key order matters!
"#.to_string(),
            content_type: "text/markdown".to_string(),
            inject_when: vec![],
            keywords: vec!["hash".to_string(), "sha256".to_string(), "compute".to_string()],
            risk_tiers: vec!["high".to_string()],
        },
        AtlasContextBlock {
            context_id: "chain-verification-guide".to_string(),
            name: "Chain Verification Guide".to_string(),
            priority: 180,
            content: r#"# Hash Chain Verification

The TRACE hash chain links events cryptographically:
```
event[0].hash = f(event[0].data, GENESIS)
event[1].hash = f(event[1].data, event[0].hash)
event[2].hash = f(event[2].data, event[1].hash)
...
```

## Verification
```rust
let result = resolver.verify_chain(&session_id)?;
assert!(result.is_valid);
```

If verification fails, check:
1. Did you use compute_hash()?
2. Did you use canonical_json()?
3. Are sequences monotonic?
"#.to_string(),
            content_type: "text/markdown".to_string(),
            inject_when: vec![],
            keywords: vec!["chain".to_string(), "verify".to_string(), "integrity".to_string()],
            risk_tiers: vec![],
        },
        AtlasContextBlock {
            context_id: "read-before-modify".to_string(),
            name: "Read Before Modify".to_string(),
            priority: 250,
            content: r#"# MANDATORY: Read Before Modify

You MUST read any file before modifying it.

Always read trace/event.rs before touching hash logic.
It contains compute_hash() - the ONLY valid hash implementation.
"#.to_string(),
            content_type: "text/markdown".to_string(),
            inject_when: vec![],
            keywords: vec!["modify".to_string(), "read".to_string(), "before".to_string()],
            risk_tiers: vec![],
        },
    ];

    atlas
}

/// Load the full self-governance atlas
fn load_self_governance_atlas() -> AtlasManifest {
    let atlas_json = include_str!("../../atlases/cra-development.json");
    serde_json::from_str(atlas_json).expect("Failed to parse cra-development.json")
}

#[test]
fn demo_without_context() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║         SCENARIO A: Agent WITHOUT Context Injection              ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let mut resolver = Resolver::new();
    let atlas = create_no_context_atlas();
    resolver.load_atlas(atlas).unwrap();

    let session_id = resolver
        .create_session("naive-agent", "No context session")
        .unwrap();

    let request = CARPRequest::new(
        session_id.clone(),
        "naive-agent".to_string(),
        "I need to add a new hash computation function to the trace module".to_string(),
    );

    println!("\n📋 REQUEST:");
    println!("   Agent: {}", request.agent_id);
    println!("   Goal: {}", request.goal);

    let resolution = resolver.resolve(&request).unwrap();

    println!("\n📤 RESOLUTION:");
    println!("   Decision: {:?}", resolution.decision);
    println!("   Allowed actions: {}", resolution.allowed_actions.len());
    println!("   Context blocks: {}", resolution.context_blocks.len());

    if resolution.context_blocks.is_empty() {
        println!("\n⚠️  NO CONTEXT INJECTED!");
        println!("   The agent receives NO guidance about hash computation.");
        println!("   They might reimplement hash logic and break the chain.");
        println!("   This is the OLD behavior - agent flies blind.");
    }

    // Show the trace
    let trace = resolver.get_trace(&session_id).unwrap();
    println!("\n📊 TRACE EVENTS ({} total):", trace.len());
    for event in &trace {
        println!("   [{:2}] {} | hash: {}...",
            event.sequence,
            event.event_type,
            &event.event_hash[..16]
        );
    }

    assert!(resolution.context_blocks.is_empty(), "No context atlas should have 0 context blocks");
}

#[test]
fn demo_with_context() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║          SCENARIO B: Agent WITH Context Injection                ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let mut resolver = Resolver::new();
    let atlas = create_with_context_atlas();
    resolver.load_atlas(atlas).unwrap();

    let session_id = resolver
        .create_session("guided-agent", "Context-guided session")
        .unwrap();

    let request = CARPRequest::new(
        session_id.clone(),
        "guided-agent".to_string(),
        "I need to add a new hash computation function to the trace module".to_string(),
    );

    println!("\n📋 REQUEST (same goal as Scenario A):");
    println!("   Agent: {}", request.agent_id);
    println!("   Goal: {}", request.goal);

    let resolution = resolver.resolve(&request).unwrap();

    println!("\n📤 RESOLUTION:");
    println!("   Decision: {:?}", resolution.decision);
    println!("   Allowed actions: {}", resolution.allowed_actions.len());
    println!("   Context blocks: {}", resolution.context_blocks.len());

    if !resolution.context_blocks.is_empty() {
        println!("\n✅ CONTEXT INJECTED!");
        println!("   The agent now receives critical guidance:\n");

        for (i, block) in resolution.context_blocks.iter().enumerate() {
            println!("   ╭─────────────────────────────────────────────────────────╮");
            println!("   │ CONTEXT BLOCK #{}: {} (priority {})", i+1, block.block_id, block.priority);
            println!("   ├─────────────────────────────────────────────────────────┤");

            // Print content with proper indentation
            for line in block.content.lines().take(10) {
                println!("   │ {}", line);
            }
            if block.content.lines().count() > 10 {
                println!("   │ ... [truncated]");
            }
            println!("   ╰─────────────────────────────────────────────────────────╯\n");
        }
    }

    // Show the trace with context.injected events
    let trace = resolver.get_trace(&session_id).unwrap();
    println!("\n📊 TRACE EVENTS ({} total):", trace.len());
    for event in &trace {
        let marker = if event.event_type == EventType::ContextInjected { "⭐" } else { "  " };
        println!("   {} [{:2}] {} | hash: {}...",
            marker,
            event.sequence,
            event.event_type,
            &event.event_hash[..16]
        );

        if event.event_type == EventType::ContextInjected {
            if let Some(ctx_id) = event.payload.get("context_id") {
                println!("          └─ Injected: {}", ctx_id);
            }
        }
    }

    // Verify chain
    let verification = resolver.verify_chain(&session_id).unwrap();
    println!("\n🔒 CHAIN VERIFICATION: {}",
        if verification.is_valid { "✅ VALID" } else { "❌ INVALID" }
    );

    assert!(!resolution.context_blocks.is_empty(), "Should have context blocks");
}

#[test]
fn demo_full_self_governance() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║       SCENARIO C: Full Self-Governance Atlas (13 contexts)       ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let mut resolver = Resolver::new();
    let atlas = load_self_governance_atlas();

    println!("\n📦 LOADED ATLAS: {}", atlas.name);
    println!("   Version: {}", atlas.version);
    println!("   Context blocks defined: {}", atlas.context_blocks.len());
    println!("\n   Available contexts:");
    for block in &atlas.context_blocks {
        println!("     - {} (priority: {}, keywords: {:?})",
            block.context_id, block.priority, block.keywords);
    }

    resolver.load_atlas(atlas).unwrap();

    let session_id = resolver
        .create_session("claude-agent", "CRA Development Session")
        .unwrap();

    // Test various goals and show what context gets injected
    let test_goals = vec![
        "I need to modify the hash computation in trace/event.rs",
        "Help me understand the CRA architecture",
        "I want to add a new matching condition to the context registry",
        "Working on deferred tracing performance",
        "Need to edit the resolver module",
    ];

    for goal in test_goals {
        println!("\n─────────────────────────────────────────────────────────────────");
        println!("🎯 GOAL: {}", goal);

        let request = CARPRequest::new(
            session_id.clone(),
            "claude-agent".to_string(),
            goal.to_string(),
        );

        let resolution = resolver.resolve(&request).unwrap();

        println!("   Injected {} context blocks:", resolution.context_blocks.len());
        for block in &resolution.context_blocks {
            println!("     ✓ {} (priority {})", block.block_id, block.priority);
        }

        if resolution.context_blocks.is_empty() {
            println!("     (no matching contexts for this goal)");
        }
    }

    // Final trace summary
    let trace = resolver.get_trace(&session_id).unwrap();
    let context_events: Vec<_> = trace.iter()
        .filter(|e| e.event_type == EventType::ContextInjected)
        .collect();

    println!("\n═══════════════════════════════════════════════════════════════════");
    println!("📊 FINAL STATISTICS:");
    println!("   Total TRACE events: {}", trace.len());
    println!("   context.injected events: {}", context_events.len());

    let verification = resolver.verify_chain(&session_id).unwrap();
    println!("   Chain integrity: {}",
        if verification.is_valid { "✅ VALID" } else { "❌ INVALID" }
    );

    // Show a sample context.injected event
    if let Some(event) = context_events.first() {
        println!("\n📝 Sample context.injected event:");
        println!("{}", serde_json::to_string_pretty(&event.payload).unwrap());
    }

    assert!(verification.is_valid);
}

#[test]
fn demo_side_by_side_comparison() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║           SIDE-BY-SIDE: Without vs With Context                  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let goal = "I need to modify the hash computation logic";

    // WITHOUT context
    let mut resolver_no_ctx = Resolver::new();
    resolver_no_ctx.load_atlas(create_no_context_atlas()).unwrap();
    let session_no_ctx = resolver_no_ctx.create_session("agent", "test").unwrap();
    let request_no_ctx = CARPRequest::new(session_no_ctx.clone(), "agent".to_string(), goal.to_string());
    let resolution_no_ctx = resolver_no_ctx.resolve(&request_no_ctx).unwrap();

    // WITH context
    let mut resolver_ctx = Resolver::new();
    resolver_ctx.load_atlas(create_with_context_atlas()).unwrap();
    let session_ctx = resolver_ctx.create_session("agent", "test").unwrap();
    let request_ctx = CARPRequest::new(session_ctx.clone(), "agent".to_string(), goal.to_string());
    let resolution_ctx = resolver_ctx.resolve(&request_ctx).unwrap();

    println!("\n🎯 Goal: \"{}\"", goal);
    println!("\n┌─────────────────────────────┬─────────────────────────────┐");
    println!("│      WITHOUT CONTEXT        │       WITH CONTEXT          │");
    println!("├─────────────────────────────┼─────────────────────────────┤");
    println!("│ Context blocks: {:>11} │ Context blocks: {:>11} │",
        resolution_no_ctx.context_blocks.len(),
        resolution_ctx.context_blocks.len()
    );
    println!("├─────────────────────────────┼─────────────────────────────┤");
    println!("│                             │                             │");
    println!("│  Agent receives:            │  Agent receives:            │");
    println!("│  - Just allowed actions     │  - Allowed actions          │");
    println!("│  - NO guidance              │  - CRITICAL warnings        │");
    println!("│  - NO best practices        │  - Code patterns            │");
    println!("│                             │  - Anti-patterns to avoid   │");
    println!("│                             │                             │");
    println!("│  Result:                    │  Result:                    │");
    println!("│  Might reimplement hash     │  Uses compute_hash()        │");
    println!("│  Chain verification fails   │  Chain stays valid          │");
    println!("│  Security broken            │  Security maintained        │");
    println!("│                             │                             │");
    println!("└─────────────────────────────┴─────────────────────────────┘");

    println!("\n📋 INJECTED CONTEXTS (with-context scenario):");
    for block in &resolution_ctx.context_blocks {
        println!("\n   [{}]", block.block_id);
        println!("   Content preview:");
        for line in block.content.lines().take(5) {
            println!("   │ {}", line);
        }
        println!("   │ ...");
    }

    assert!(resolution_no_ctx.context_blocks.is_empty());
    assert!(!resolution_ctx.context_blocks.is_empty());
}
