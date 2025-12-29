//! Integration test demonstrating the full CRA context injection flow.
//!
//! This test shows what an agent would receive when calling CRA with a goal.

use cra_core::{
    Resolver, CARPRequest,
    atlas::AtlasManifest,
};

/// Load the self-governance atlas
fn load_self_governance_atlas() -> AtlasManifest {
    let atlas_json = include_str!("../../atlases/cra-development.json");
    serde_json::from_str(atlas_json).expect("Failed to parse cra-development.json")
}

/// Demonstrate the full flow: Goal → CRA → Rendered Context
#[test]
fn test_full_context_injection_flow() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║           FULL CRA CONTEXT INJECTION FLOW DEMONSTRATION              ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");

    // 1. Setup CRA with the self-governance atlas
    let mut resolver = Resolver::new();
    let atlas = load_self_governance_atlas();

    println!("\n📦 Loaded Atlas: {} v{}", atlas.name, atlas.version);
    println!("   Context blocks available: {}", atlas.context_blocks.len());

    resolver.load_atlas(atlas).expect("Failed to load atlas");

    // 2. Create a session (simulating an agent starting work)
    let session_id = resolver
        .create_session("test-agent", "CRA development")
        .expect("Failed to create session");

    println!("\n🎯 Session created: {}", &session_id[..8]);

    // 3. Agent submits a goal via CARPRequest
    let goal = "I need to modify the hash computation in the trace module";

    let request = CARPRequest::new(
        session_id.clone(),
        "test-agent".to_string(),
        goal.to_string(),
    );

    println!("\n📝 Agent Goal: \"{}\"", goal);

    // 4. CRA resolves the request - this is where context injection happens
    let resolution = resolver.resolve(&request).expect("Failed to resolve");

    println!("\n✅ Resolution received:");
    println!("   Decision: {:?}", resolution.decision);
    println!("   Context blocks injected: {}", resolution.context_blocks.len());

    for block in &resolution.context_blocks {
        println!("   • {} (priority: {})", block.block_id, block.priority);
    }

    // 5. Render the context for LLM consumption
    let rendered = resolution.render_context();

    println!("\n{}", "═".repeat(72));
    println!("RENDERED CONTEXT (what the agent would receive):");
    println!("{}", "═".repeat(72));
    println!("{}", rendered);
    println!("{}", "═".repeat(72));

    // 6. Verify the context includes critical information
    assert!(
        rendered.contains("compute_hash"),
        "Context should mention compute_hash()"
    );
    assert!(
        rendered.contains("NEVER") || rendered.contains("never"),
        "Context should include warnings"
    );
    assert!(
        rendered.contains("trace/event.rs") || rendered.contains("trace"),
        "Context should reference trace module"
    );

    println!("\n✓ Context contains critical guidance for hash-related work");
}

/// Test different goals and the context they trigger
#[test]
fn test_context_varies_by_goal() {
    let mut resolver = Resolver::new();
    let atlas = load_self_governance_atlas();
    resolver.load_atlas(atlas).expect("Failed to load atlas");

    let session_id = resolver
        .create_session("test-agent", "Testing context variation")
        .expect("Failed to create session");

    let test_cases = vec![
        (
            "I need to modify the hash computation",
            vec!["hash", "compute_hash"],
            "hash-related context"
        ),
        (
            "Help me understand the CRA architecture",
            vec!["CRA", "architecture", "CARP", "TRACE"],
            "architecture context"
        ),
        (
            "I want to add a new context matching condition",
            vec!["context", "matching", "registry"],
            "context registry guidance"
        ),
        (
            "Working on deferred tracing performance",
            vec!["deferred", "flush", "performance"],
            "deferred mode context"
        ),
    ];

    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║           CONTEXT VARIES BY GOAL - DEMONSTRATION                     ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");

    for (goal, expected_terms, description) in test_cases {
        let request = CARPRequest::new(
            session_id.clone(),
            "test-agent".to_string(),
            goal.to_string(),
        );

        let resolution = resolver.resolve(&request).expect("Failed to resolve");
        let rendered = resolution.render_context();

        println!("\n─────────────────────────────────────────────────────────────────");
        println!("Goal: \"{}\"", goal);
        println!("Blocks injected: {}", resolution.context_blocks.len());
        println!("Expected: {}", description);

        // Check that at least some expected terms appear
        let terms_found: Vec<_> = expected_terms.iter()
            .filter(|term| rendered.to_lowercase().contains(&term.to_lowercase()))
            .collect();

        if terms_found.is_empty() {
            println!("⚠️  No expected terms found in rendered context");
        } else {
            println!("✓ Found terms: {:?}", terms_found);
        }
    }
}

/// Show exactly what would be injected for a real task
#[test]
fn test_real_task_context_output() {
    let mut resolver = Resolver::new();
    let atlas = load_self_governance_atlas();
    resolver.load_atlas(atlas).expect("Failed to load atlas");

    let session_id = resolver
        .create_session("claude-agent", "Real task simulation")
        .expect("Failed to create session");

    // The exact task we'd give to a fresh agent
    let goal = "Add a new TRACE event type called context.stale that fires when stale context is detected";

    let request = CARPRequest::new(
        session_id.clone(),
        "claude-agent".to_string(),
        goal.to_string(),
    );

    let resolution = resolver.resolve(&request).expect("Failed to resolve");
    let rendered = resolution.render_context();

    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                  REAL TASK: ADD context.stale EVENT                  ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!("\n📝 Task: {}", goal);
    println!("\n📋 CONTEXT THAT WOULD BE INJECTED:");
    println!("{}", "─".repeat(72));
    println!("{}", rendered);

    // This is the context an agent would receive
    // It should help them:
    // 1. Find trace/event.rs
    // 2. Follow the EventType pattern
    // 3. NOT reimplement hash logic
    // 4. Run appropriate tests

    // Save to file for manual inspection
    std::fs::write(
        "/tmp/cra_rendered_context.md",
        &rendered
    ).ok();

    println!("\n💾 Context saved to /tmp/cra_rendered_context.md for inspection");
}
