//! VIB3+ Atlas Context Injection Demonstration
//!
//! This test demonstrates the difference between:
//! - An agent working WITHOUT VIB3+ context (makes common mistakes)
//! - An agent working WITH VIB3+ context (gets proper guidance)

use cra_core::{
    Resolver, CARPRequest,
    atlas::AtlasManifest,
    trace::EventType,
};
use serde_json::json;

/// Create a minimal atlas WITHOUT VIB3+ context blocks
fn create_no_vib3_context_atlas() -> AtlasManifest {
    serde_json::from_value(json!({
        "atlas_version": "1.0",
        "atlas_id": "demo.no-vib3",
        "version": "1.0.0",
        "name": "No VIB3 Context Atlas",
        "description": "Atlas with NO VIB3 context - agent has no guidance",
        "domains": ["webgl", "visualization"],
        "capabilities": [],
        "policies": [],
        "actions": [
            {
                "action_id": "code.write",
                "name": "Write Code",
                "description": "Write source code",
                "parameters_schema": {"type": "object"},
                "risk_tier": "low"
            }
        ]
    })).unwrap()
}

/// Load the full VIB3+ development atlas
fn load_vib3_atlas() -> AtlasManifest {
    let atlas_json = include_str!("../../atlases/vib3-webpage-development.json");
    serde_json::from_str(atlas_json).expect("Failed to parse vib3-webpage-development.json")
}

#[test]
fn demo_vib3_without_context() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║     SCENARIO A: Agent WITHOUT VIB3+ Context Injection            ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let mut resolver = Resolver::new();
    let atlas = create_no_vib3_context_atlas();
    resolver.load_atlas(atlas).unwrap();

    let session_id = resolver
        .create_session("naive-agent", "No VIB3 context session")
        .unwrap();

    let request = CARPRequest::new(
        session_id.clone(),
        "naive-agent".to_string(),
        "Add a 4D polychora visualization to the webpage background".to_string(),
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
        println!("   The agent receives NO guidance about VIB3+.");
        println!("   Common mistakes an agent might make:");
        println!("   ❌ Try to use 'polychora' system (PLACEHOLDER ONLY, doesn't work!)");
        println!("   ❌ Try to render actual 4D wireframe geometry");
        println!("   ❌ Not know which systems actually work (faceted, quantum, holographic)");
        println!("   ❌ Miss the iframe embedding approach entirely");
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
fn demo_vib3_with_context() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║      SCENARIO B: Agent WITH VIB3+ Context Injection              ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let mut resolver = Resolver::new();
    let atlas = load_vib3_atlas();

    println!("\n📦 LOADED ATLAS: {}", atlas.name);
    println!("   Version: {}", atlas.version);
    println!("   Context blocks defined: {}", atlas.context_blocks.len());

    if let Some(sources) = &atlas.sources {
        println!("   Demo URL: {:?}", sources.demo);
    }

    resolver.load_atlas(atlas).unwrap();

    let session_id = resolver
        .create_session("guided-agent", "VIB3 context-guided session")
        .unwrap();

    let request = CARPRequest::new(
        session_id.clone(),
        "guided-agent".to_string(),
        "Add a 4D polychora visualization to the webpage background".to_string(),
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
            for line in block.content.lines().take(15) {
                println!("   │ {}", line);
            }
            if block.content.lines().count() > 15 {
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

    assert!(verification.is_valid);
}

#[test]
fn demo_vib3_various_goals() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║       SCENARIO C: Various VIB3+ Goals & Context Matching         ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let mut resolver = Resolver::new();
    let atlas = load_vib3_atlas();

    println!("\n📦 LOADED ATLAS: {}", atlas.name);
    println!("   Context blocks available:");
    for block in &atlas.context_blocks {
        println!("     - {} (priority: {}, inject_mode: {:?})",
            block.context_id, block.priority, block.inject_mode);
    }

    resolver.load_atlas(atlas).unwrap();

    let session_id = resolver
        .create_session("vib3-developer", "VIB3+ Development Session")
        .unwrap();

    // Test various goals and show what context gets injected
    let test_goals = vec![
        "Embed a shader background into my landing page",
        "Build a music visualizer with VIB3+",
        "Customize the VIB3+ UI controls",
        "Use the SDK to add touch interaction",
        "What geometry index should I use for torus?",
        "Clone the repo and modify the defaults",
    ];

    for goal in test_goals {
        println!("\n─────────────────────────────────────────────────────────────────");
        println!("🎯 GOAL: {}", goal);

        let request = CARPRequest::new(
            session_id.clone(),
            "vib3-developer".to_string(),
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

    assert!(verification.is_valid);
}

#[test]
fn demo_vib3_side_by_side() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║        SIDE-BY-SIDE: Without vs With VIB3+ Context               ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let goal = "Add animated shader background to my portfolio site";

    // WITHOUT context
    let mut resolver_no_ctx = Resolver::new();
    resolver_no_ctx.load_atlas(create_no_vib3_context_atlas()).unwrap();
    let session_no_ctx = resolver_no_ctx.create_session("agent", "test").unwrap();
    let request_no_ctx = CARPRequest::new(session_no_ctx.clone(), "agent".to_string(), goal.to_string());
    let resolution_no_ctx = resolver_no_ctx.resolve(&request_no_ctx).unwrap();

    // WITH context
    let mut resolver_ctx = Resolver::new();
    resolver_ctx.load_atlas(load_vib3_atlas()).unwrap();
    let session_ctx = resolver_ctx.create_session("agent", "test").unwrap();
    let request_ctx = CARPRequest::new(session_ctx.clone(), "agent".to_string(), goal.to_string());
    let resolution_ctx = resolver_ctx.resolve(&request_ctx).unwrap();

    println!("\n🎯 Goal: \"{}\"", goal);
    println!("\n┌────────────────────────────────┬────────────────────────────────┐");
    println!("│       WITHOUT VIB3+ ATLAS      │        WITH VIB3+ ATLAS        │");
    println!("├────────────────────────────────┼────────────────────────────────┤");
    println!("│ Context blocks: {:>14} │ Context blocks: {:>14} │",
        resolution_no_ctx.context_blocks.len(),
        resolution_ctx.context_blocks.len()
    );
    println!("├────────────────────────────────┼────────────────────────────────┤");
    println!("│                                │                                │");
    println!("│  Agent receives:               │  Agent receives:               │");
    println!("│  - NO shader guidance          │  - Working systems list        │");
    println!("│  - NO embedding patterns       │  - Iframe embed code           │");
    println!("│  - NO geometry reference       │  - URL parameters              │");
    println!("│                                │  - Geometry formula            │");
    println!("│                                │                                │");
    println!("│  Likely mistakes:              │  Correct behavior:             │");
    println!("│  ❌ Try 'polychora' (broken)   │  ✅ Use holographic/quantum    │");
    println!("│  ❌ Build shader from scratch  │  ✅ Embed via iframe           │");
    println!("│  ❌ Wrong geometry indices     │  ✅ Use geometry formula       │");
    println!("│                                │                                │");
    println!("└────────────────────────────────┴────────────────────────────────┘");

    if !resolution_ctx.context_blocks.is_empty() {
        println!("\n📋 INJECTED CONTEXTS (with VIB3+ atlas):");
        for block in &resolution_ctx.context_blocks {
            println!("\n   [{}]", block.block_id);
            println!("   Content preview:");
            for line in block.content.lines().take(8) {
                println!("   │ {}", line);
            }
            println!("   │ ...");
        }
    }

    // Verify both chains
    let verify_no_ctx = resolver_no_ctx.verify_chain(&session_no_ctx).unwrap();
    let verify_ctx = resolver_ctx.verify_chain(&session_ctx).unwrap();

    println!("\n🔒 CHAIN VERIFICATION:");
    println!("   Without context: {}", if verify_no_ctx.is_valid { "✅ VALID" } else { "❌ INVALID" });
    println!("   With context:    {}", if verify_ctx.is_valid { "✅ VALID" } else { "❌ INVALID" });

    assert!(resolution_no_ctx.context_blocks.is_empty());
    assert!(verify_no_ctx.is_valid);
    assert!(verify_ctx.is_valid);
}
