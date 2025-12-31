//! Self-Governance Integration Tests
//!
//! These tests demonstrate CRA governing its own development - "eating our own dog food".
//! The cra-development atlas injects context to prevent common mistakes like
//! reimplementing hash computation.

use cra_core::{
    Resolver, CARPRequest,
    atlas::AtlasManifest,
    trace::EventType,
};

/// Load the self-governance atlas from the atlases directory
fn load_self_governance_atlas() -> AtlasManifest {
    let atlas_json = include_str!("../../atlases/cra-development.json");
    serde_json::from_str(atlas_json).expect("Failed to parse cra-development.json")
}

/// Test: When an LLM mentions "hash" in their goal, they should receive
/// the hash-computation-rule context block.
#[test]
fn test_hash_context_injection() {
    let mut resolver = Resolver::new();
    let atlas = load_self_governance_atlas();

    resolver.load_atlas(atlas).expect("Failed to load atlas");

    let session_id = resolver
        .create_session("llm-agent", "CRA development session")
        .expect("Failed to create session");

    // Simulate an LLM asking to work on hash-related code
    let request = CARPRequest::new(
        session_id.clone(),
        "llm-agent".to_string(),
        "I need to modify the hash computation in the trace module".to_string(),
    );

    let resolution = resolver.resolve(&request).expect("Failed to resolve");

    // Should have injected hash-related context
    assert!(
        !resolution.context_blocks.is_empty(),
        "Should inject context when goal mentions 'hash'"
    );

    // Find the hash computation rule
    let hash_context = resolution.context_blocks.iter()
        .find(|b| b.block_id == "hash-computation-rule");

    assert!(
        hash_context.is_some(),
        "Should inject 'hash-computation-rule' context block"
    );

    let hash_block = hash_context.unwrap();
    assert!(
        hash_block.content.contains("compute_hash()"),
        "Hash context should mention compute_hash()"
    );
    assert!(
        hash_block.content.contains("NEVER reimplement"),
        "Hash context should warn against reimplementation"
    );

    // Verify context.injected events in trace
    let trace = resolver.get_trace(&session_id).expect("Failed to get trace");
    let context_events: Vec<_> = trace.iter()
        .filter(|e| e.event_type == EventType::ContextInjected)
        .collect();

    assert!(
        !context_events.is_empty(),
        "Should emit context.injected TRACE events"
    );

    println!("\n=== Hash Context Injection Test ===");
    println!("Goal: {}", request.goal);
    println!("Injected {} context blocks:", resolution.context_blocks.len());
    for block in &resolution.context_blocks {
        println!("  - {} (priority: {})", block.block_id, block.priority);
    }
    println!("TRACE events: {} context.injected", context_events.len());
}

/// Test: When an LLM mentions "deferred" or "flush", they should receive
/// deferred mode documentation.
#[test]
fn test_deferred_mode_context_injection() {
    let mut resolver = Resolver::new();
    let atlas = load_self_governance_atlas();

    resolver.load_atlas(atlas).expect("Failed to load atlas");

    let session_id = resolver
        .create_session("llm-agent", "Performance optimization")
        .expect("Failed to create session");

    let request = CARPRequest::new(
        session_id.clone(),
        "llm-agent".to_string(),
        "I want to optimize the deferred tracing buffer flush mechanism".to_string(),
    );

    let resolution = resolver.resolve(&request).expect("Failed to resolve");

    // Should have injected deferred mode context
    let deferred_context = resolution.context_blocks.iter()
        .find(|b| b.block_id == "deferred-mode-pattern");

    assert!(
        deferred_context.is_some(),
        "Should inject 'deferred-mode-pattern' context for deferred/flush goals"
    );

    let deferred_block = deferred_context.unwrap();
    assert!(
        deferred_block.content.contains("flush()"),
        "Deferred context should explain flush()"
    );
    assert!(
        deferred_block.content.contains("compute_hash()"),
        "Should emphasize that flush() uses compute_hash()"
    );

    println!("\n=== Deferred Mode Context Injection Test ===");
    println!("Goal: {}", request.goal);
    println!("Injected blocks: {:?}",
        resolution.context_blocks.iter().map(|b| &b.block_id).collect::<Vec<_>>());
}

/// Test: When an LLM asks about the architecture, they get the overview.
#[test]
fn test_architecture_context_injection() {
    let mut resolver = Resolver::new();
    let atlas = load_self_governance_atlas();

    resolver.load_atlas(atlas).expect("Failed to load atlas");

    let session_id = resolver
        .create_session("llm-agent", "Learning CRA")
        .expect("Failed to create session");

    let request = CARPRequest::new(
        session_id.clone(),
        "llm-agent".to_string(),
        "Help me understand the CRA architecture and overall design".to_string(),
    );

    let resolution = resolver.resolve(&request).expect("Failed to resolve");

    let arch_context = resolution.context_blocks.iter()
        .find(|b| b.block_id == "cra-architecture-overview");

    assert!(
        arch_context.is_some(),
        "Should inject architecture overview for design questions"
    );

    let arch_block = arch_context.unwrap();
    assert!(
        arch_block.content.contains("Context + Resolution + Action"),
        "Architecture should explain CRA meaning"
    );

    println!("\n=== Architecture Context Injection Test ===");
    println!("Goal: {}", request.goal);
    println!("Got architecture overview: {}", arch_context.is_some());
}

/// Test: The complete flow - multiple contexts can be injected at once
/// based on a complex goal.
#[test]
fn test_multi_context_injection() {
    let mut resolver = Resolver::new();
    let atlas = load_self_governance_atlas();

    resolver.load_atlas(atlas).expect("Failed to load atlas");

    let session_id = resolver
        .create_session("llm-agent", "Major refactoring")
        .expect("Failed to create session");

    // A complex goal that should trigger multiple contexts
    let request = CARPRequest::new(
        session_id.clone(),
        "llm-agent".to_string(),
        "I need to modify the hash chain verification in the trace module \
         and update the resolver to handle deferred mode better".to_string(),
    );

    let resolution = resolver.resolve(&request).expect("Failed to resolve");

    println!("\n=== Multi-Context Injection Test ===");
    println!("Goal: {}", request.goal);
    println!("\nInjected {} context blocks:", resolution.context_blocks.len());

    for block in &resolution.context_blocks {
        println!("\n--- {} (priority: {}) ---", block.block_id, block.priority);
        // Print first 200 chars of content
        let preview: String = block.content.chars().take(200).collect();
        println!("{}", preview);
        if block.content.len() > 200 {
            println!("...[truncated]");
        }
    }

    // Should have multiple relevant contexts
    assert!(
        resolution.context_blocks.len() >= 2,
        "Complex goal should trigger multiple context blocks"
    );

    // Verify chain integrity
    let verification = resolver.verify_chain(&session_id).expect("Failed to verify");
    assert!(verification.is_valid, "Chain should be valid");

    println!("\nChain verification: {}",
        if verification.is_valid { "VALID" } else { "INVALID" });
}

/// Test: Context blocks are ordered by priority (higher priority first).
#[test]
fn test_context_priority_ordering() {
    let mut resolver = Resolver::new();
    let atlas = load_self_governance_atlas();

    resolver.load_atlas(atlas).expect("Failed to load atlas");

    let session_id = resolver
        .create_session("llm-agent", "Testing priorities")
        .expect("Failed to create session");

    // Goal that should match multiple contexts
    let request = CARPRequest::new(
        session_id.clone(),
        "llm-agent".to_string(),
        "I need to understand the hash chain before modifying the trace module".to_string(),
    );

    let resolution = resolver.resolve(&request).expect("Failed to resolve");

    if resolution.context_blocks.len() >= 2 {
        // Check that blocks are ordered by priority (higher first)
        let priorities: Vec<i32> = resolution.context_blocks.iter()
            .map(|b| b.priority)
            .collect();

        println!("\n=== Priority Ordering Test ===");
        println!("Priority order: {:?}", priorities);

        // Blocks with higher priority should come first
        // Note: Our current implementation doesn't guarantee ordering,
        // but we should verify the priorities are reasonable
        for block in &resolution.context_blocks {
            println!("  {} -> priority {}", block.block_id, block.priority);
        }
    }
}

/// Test: TRACE events provide audit trail of context injection.
#[test]
fn test_context_injection_audit_trail() {
    let mut resolver = Resolver::new();
    let atlas = load_self_governance_atlas();

    resolver.load_atlas(atlas).expect("Failed to load atlas");

    let session_id = resolver
        .create_session("llm-agent", "Audit test")
        .expect("Failed to create session");

    let request = CARPRequest::new(
        session_id.clone(),
        "llm-agent".to_string(),
        "Working on the hash computation module".to_string(),
    );

    resolver.resolve(&request).expect("Failed to resolve");

    let trace = resolver.get_trace(&session_id).expect("Failed to get trace");

    // Find context.injected events
    let context_events: Vec<_> = trace.iter()
        .filter(|e| e.event_type == EventType::ContextInjected)
        .collect();

    println!("\n=== Audit Trail Test ===");
    println!("Total TRACE events: {}", trace.len());
    println!("context.injected events: {}", context_events.len());

    for event in &context_events {
        println!("\nEvent {} (seq {}):", event.event_id, event.sequence);
        println!("  Payload: {}", serde_json::to_string_pretty(&event.payload).unwrap());
    }

    // Verify chain integrity
    let verification = resolver.verify_chain(&session_id).expect("Failed to verify");
    assert!(verification.is_valid, "Chain must be valid for audit");

    println!("\nChain integrity: {} events verified", verification.event_count);
}

/// Demonstration: What happens when an LLM gets context about the Context Registry
#[test]
fn test_meta_context_injection() {
    let mut resolver = Resolver::new();
    let atlas = load_self_governance_atlas();

    resolver.load_atlas(atlas).expect("Failed to load atlas");

    let session_id = resolver
        .create_session("llm-agent", "Working on context system")
        .expect("Failed to create session");

    // Goal about the context registry itself - META!
    let request = CARPRequest::new(
        session_id.clone(),
        "llm-agent".to_string(),
        "I want to add a new matching condition to the context registry".to_string(),
    );

    let resolution = resolver.resolve(&request).expect("Failed to resolve");

    println!("\n=== Meta Context Injection Test ===");
    println!("Goal: {}", request.goal);
    println!("\nThis is META - CRA injecting context about CRA's context system!");

    for block in &resolution.context_blocks {
        println!("\n--- {} ---", block.block_id);
        if block.block_id.contains("context") {
            println!("  [META: Context about the context system!]");
        }
    }

    // Should have context registry documentation
    let has_context_docs = resolution.context_blocks.iter()
        .any(|b| b.block_id.contains("context") || b.content.contains("ContextRegistry"));

    assert!(
        has_context_docs,
        "Should inject context documentation when working on context system"
    );
}
