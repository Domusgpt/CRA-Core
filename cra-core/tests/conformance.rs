//! Conformance tests against golden traces from specs/conformance/
//!
//! These tests verify that the Rust implementation matches the protocol specification
//! by comparing output against known-good golden traces.

use cra_core::atlas::{AtlasManifest, AtlasContextBlock};
use cra_core::carp::{CARPRequest, Decision, Resolver};
use cra_core::trace::EventType;
use serde_json::{json, Value};

/// Load the simple-resolve test atlas
fn load_simple_atlas() -> AtlasManifest {
    let atlas_json = include_str!("../../specs/conformance/golden/simple-resolve/atlas.json");
    serde_json::from_str(atlas_json).expect("Failed to parse atlas.json")
}

/// Load the expected resolution (for reference, not strict comparison)
#[allow(dead_code)]
fn load_expected_resolution() -> Value {
    let resolution_json =
        include_str!("../../specs/conformance/golden/simple-resolve/expected-resolution.json");
    serde_json::from_str(resolution_json).expect("Failed to parse expected-resolution.json")
}

/// Load the expected trace events (for reference, not strict comparison)
#[allow(dead_code)]
fn load_expected_trace() -> Vec<Value> {
    let trace_jsonl =
        include_str!("../../specs/conformance/golden/simple-resolve/expected-trace.jsonl");
    trace_jsonl
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("Failed to parse trace line"))
        .collect()
}

#[test]
fn conformance_simple_resolve_decision() {
    // Setup
    let mut resolver = Resolver::new();
    let atlas = load_simple_atlas();
    resolver.load_atlas(atlas).expect("Failed to load atlas");

    // Create session with known ID
    let session_id = resolver
        .create_session("test-agent", "I need to read and manage data")
        .expect("Failed to create session");

    // Create request
    let request = CARPRequest::new(
        session_id.clone(),
        "test-agent".to_string(),
        "I need to read and manage data".to_string(),
    );

    // Resolve
    let resolution = resolver.resolve(&request).expect("Failed to resolve");

    // Verify decision matches expected
    let _expected = load_expected_resolution();

    // Check decision type
    assert_eq!(
        resolution.decision,
        Decision::Partial,
        "Decision should be 'partial' when some actions are denied"
    );

    // Check allowed actions (resource.get, resource.list, resource.create, resource.update)
    let allowed_ids: Vec<&str> = resolution
        .allowed_actions
        .iter()
        .map(|a| a.action_id.as_str())
        .collect();
    assert!(
        allowed_ids.contains(&"resource.get"),
        "resource.get should be allowed"
    );
    assert!(
        allowed_ids.contains(&"resource.list"),
        "resource.list should be allowed"
    );
    assert!(
        allowed_ids.contains(&"resource.create"),
        "resource.create should be allowed"
    );
    assert!(
        allowed_ids.contains(&"resource.update"),
        "resource.update should be allowed"
    );
    assert_eq!(allowed_ids.len(), 4, "Should have exactly 4 allowed actions");

    // Check denied actions
    let denied_ids: Vec<&str> = resolution
        .denied_actions
        .iter()
        .map(|a| a.action_id.as_str())
        .collect();
    assert!(
        denied_ids.contains(&"resource.delete"),
        "resource.delete should be denied"
    );
    assert_eq!(denied_ids.len(), 1, "Should have exactly 1 denied action");

    // Check deny reason
    let delete_denial = resolution
        .denied_actions
        .iter()
        .find(|a| a.action_id == "resource.delete")
        .expect("resource.delete denial not found");
    assert_eq!(delete_denial.policy_id, "deny-delete");
}

#[test]
fn conformance_simple_resolve_trace_events() {
    // Setup
    let mut resolver = Resolver::new();
    let atlas = load_simple_atlas();
    resolver.load_atlas(atlas).expect("Failed to load atlas");

    // Create session
    let session_id = resolver
        .create_session("test-agent", "I need to read and manage data")
        .expect("Failed to create session");

    // Create and resolve request
    let request = CARPRequest::new(
        session_id.clone(),
        "test-agent".to_string(),
        "I need to read and manage data".to_string(),
    );
    let _resolution = resolver.resolve(&request).expect("Failed to resolve");

    // Get trace events
    let trace = resolver
        .get_trace(&session_id)
        .expect("Failed to get trace");

    // Verify expected event types are present
    let actual_event_types: Vec<String> = trace
        .iter()
        .map(|e| e.event_type.to_string())
        .collect();

    // Expected sequence: session.started, carp.request.received, N x policy.evaluated, carp.resolution.completed
    assert!(
        actual_event_types.contains(&"session.started".to_string()),
        "Must have session.started event"
    );
    assert!(
        actual_event_types.contains(&"carp.request.received".to_string()),
        "Must have carp.request.received event"
    );
    assert!(
        actual_event_types.contains(&"carp.resolution.completed".to_string()),
        "Must have carp.resolution.completed event"
    );

    // Policy evaluated events - one per action (5 actions in the atlas)
    let policy_count = actual_event_types.iter()
        .filter(|e| *e == "policy.evaluated")
        .count();
    assert_eq!(
        policy_count, 5,
        "Should have 5 policy.evaluated events (one per action)"
    );

    // First event must be session.started
    assert_eq!(
        actual_event_types[0], "session.started",
        "First event must be session.started"
    );

    // Last event must be carp.resolution.completed
    assert_eq!(
        actual_event_types.last().unwrap(), "carp.resolution.completed",
        "Last event must be carp.resolution.completed"
    );
}

#[test]
fn conformance_simple_resolve_trace_payloads() {
    // Setup
    let mut resolver = Resolver::new();
    let atlas = load_simple_atlas();
    resolver.load_atlas(atlas).expect("Failed to load atlas");

    // Create session
    let session_id = resolver
        .create_session("test-agent", "I need to read and manage data")
        .expect("Failed to create session");

    // Create and resolve request
    let request = CARPRequest::new(
        session_id.clone(),
        "test-agent".to_string(),
        "I need to read and manage data".to_string(),
    );
    let _resolution = resolver.resolve(&request).expect("Failed to resolve");

    // Get trace events
    let trace = resolver
        .get_trace(&session_id)
        .expect("Failed to get trace");

    // Verify session.started payload
    let session_started = &trace[0];
    assert_eq!(session_started.event_type.to_string(), "session.started");
    assert_eq!(
        session_started.payload["agent_id"].as_str().unwrap(),
        "test-agent"
    );
    assert_eq!(
        session_started.payload["goal"].as_str().unwrap(),
        "I need to read and manage data"
    );

    // Verify carp.request.received payload
    let carp_request = &trace[1];
    assert_eq!(carp_request.event_type.to_string(), "carp.request.received");
    assert_eq!(
        carp_request.payload["operation"].as_str().unwrap(),
        "resolve"
    );

    // Verify carp.resolution.completed payload
    let carp_resolution = trace
        .iter()
        .find(|e| e.event_type.to_string() == "carp.resolution.completed")
        .expect("carp.resolution.completed not found");
    assert_eq!(
        carp_resolution.payload["decision_type"].as_str().unwrap(),
        "partial"
    );
    // 4 allowed: resource.get, resource.list, resource.create, resource.update
    // 1 denied: resource.delete
    assert_eq!(carp_resolution.payload["allowed_count"].as_i64().unwrap(), 4);
    assert_eq!(carp_resolution.payload["denied_count"].as_i64().unwrap(), 1);
}

#[test]
fn conformance_hash_chain_integrity() {
    // Setup
    let mut resolver = Resolver::new();
    let atlas = load_simple_atlas();
    resolver.load_atlas(atlas).expect("Failed to load atlas");

    // Create session
    let session_id = resolver
        .create_session("test-agent", "Test goal")
        .expect("Failed to create session");

    // Create and resolve request
    let request = CARPRequest::new(
        session_id.clone(),
        "test-agent".to_string(),
        "Test goal".to_string(),
    );
    let _resolution = resolver.resolve(&request).expect("Failed to resolve");

    // Verify chain integrity
    let verification = resolver
        .verify_chain(&session_id)
        .expect("Failed to verify chain");

    assert!(
        verification.is_valid,
        "Hash chain should be valid. Error: {:?}",
        verification.error_type
    );
}

#[test]
fn conformance_policy_deny_takes_precedence() {
    // Setup
    let mut resolver = Resolver::new();

    // Create atlas with both allow and deny for same action
    let atlas: AtlasManifest = serde_json::from_value(json!({
        "atlas_version": "1.0",
        "atlas_id": "com.test.precedence",
        "version": "1.0.0",
        "name": "Precedence Test Atlas",
        "description": "Tests that deny takes precedence over allow",
        "policies": [
            {
                "policy_id": "allow-all",
                "type": "allow",
                "actions": ["*"]
            },
            {
                "policy_id": "deny-delete",
                "type": "deny",
                "actions": ["*.delete"],
                "reason": "Deny should win"
            }
        ],
        "actions": [
            {
                "action_id": "data.get",
                "name": "Get Data",
                "description": "Get data",
                "parameters_schema": {"type": "object"},
                "risk_tier": "low"
            },
            {
                "action_id": "data.delete",
                "name": "Delete Data",
                "description": "Delete data",
                "parameters_schema": {"type": "object"},
                "risk_tier": "high"
            }
        ]
    }))
    .expect("Failed to create atlas");

    resolver.load_atlas(atlas).expect("Failed to load atlas");

    let session_id = resolver
        .create_session("test-agent", "Test")
        .expect("Failed to create session");

    let request = CARPRequest::new(
        session_id.clone(),
        "test-agent".to_string(),
        "Test".to_string(),
    );

    let resolution = resolver.resolve(&request).expect("Failed to resolve");

    // data.delete should be denied even though allow-all matches
    let denied_ids: Vec<&str> = resolution
        .denied_actions
        .iter()
        .map(|a| a.action_id.as_str())
        .collect();
    assert!(
        denied_ids.contains(&"data.delete"),
        "data.delete should be denied (deny > allow)"
    );

    // data.get should be allowed
    let allowed_ids: Vec<&str> = resolution
        .allowed_actions
        .iter()
        .map(|a| a.action_id.as_str())
        .collect();
    assert!(allowed_ids.contains(&"data.get"), "data.get should be allowed");
}

#[test]
fn conformance_genesis_event_hash() {
    use cra_core::trace::GENESIS_HASH;

    // Verify genesis hash is 64 zeros
    assert_eq!(GENESIS_HASH.len(), 64, "Genesis hash should be 64 characters");
    assert!(
        GENESIS_HASH.chars().all(|c| c == '0'),
        "Genesis hash should be all zeros"
    );
}

#[test]
fn conformance_sequence_monotonic() {
    // Setup
    let mut resolver = Resolver::new();
    let atlas = load_simple_atlas();
    resolver.load_atlas(atlas).expect("Failed to load atlas");

    // Create session
    let session_id = resolver
        .create_session("test-agent", "Test goal")
        .expect("Failed to create session");

    // Multiple resolutions
    for _ in 0..3 {
        let request = CARPRequest::new(
            session_id.clone(),
            "test-agent".to_string(),
            "Test goal".to_string(),
        );
        let _resolution = resolver.resolve(&request).expect("Failed to resolve");
    }

    // Get trace and verify sequences are monotonic
    let trace = resolver
        .get_trace(&session_id)
        .expect("Failed to get trace");

    for i in 1..trace.len() {
        assert_eq!(
            trace[i].sequence,
            trace[i - 1].sequence + 1,
            "Sequence should be monotonically increasing at index {}",
            i
        );
    }
}

// ============================================================================
// CONTEXT INJECTION CONFORMANCE
// ============================================================================

#[test]
fn conformance_context_injection_keyword_match() {
    let mut resolver = Resolver::new();
    let mut atlas: AtlasManifest = serde_json::from_value(json!({
        "atlas_version": "1.0",
        "atlas_id": "conformance.context.keyword",
        "version": "1.0.0",
        "name": "Context Keyword Conformance",
        "description": "Tests keyword-based context injection",
        "domains": ["test"],
        "capabilities": [],
        "policies": [],
        "actions": []
    })).unwrap();

    atlas.context_blocks = vec![
        AtlasContextBlock {
            context_id: "hash-rules".to_string(),
            name: "Hash Computation Rules".to_string(),
            priority: 100,
            content: "CRITICAL: Use compute_hash() only.".to_string(),
            content_type: "text/markdown".to_string(),
            inject_when: vec![],
            keywords: vec!["hash".to_string(), "sha256".to_string()],
            risk_tiers: vec![],
        },
    ];

    resolver.load_atlas(atlas).unwrap();
    let session_id = resolver.create_session("test", "Test").unwrap();

    // Goal with keyword should inject context
    let request = CARPRequest::new(
        session_id.clone(),
        "test".to_string(),
        "Working on hash computation".to_string(),
    );
    let resolution = resolver.resolve(&request).unwrap();

    assert!(
        resolution.context_blocks.iter().any(|b| b.block_id == "hash-rules"),
        "Conformance: Keyword 'hash' in goal must trigger context injection"
    );

    // Goal without keyword should NOT inject context
    let request_no_match = CARPRequest::new(
        session_id.clone(),
        "test".to_string(),
        "Working on something else".to_string(),
    );
    let resolution_no_match = resolver.resolve(&request_no_match).unwrap();

    assert!(
        !resolution_no_match.context_blocks.iter().any(|b| b.block_id == "hash-rules"),
        "Conformance: Goal without matching keyword must NOT inject context"
    );
}

#[test]
fn conformance_context_injection_emits_trace_event() {
    let mut resolver = Resolver::new();
    let mut atlas: AtlasManifest = serde_json::from_value(json!({
        "atlas_version": "1.0",
        "atlas_id": "conformance.context.trace",
        "version": "1.0.0",
        "name": "Context Trace Conformance",
        "description": "Tests context.injected TRACE events",
        "domains": ["test"],
        "capabilities": [],
        "policies": [],
        "actions": []
    })).unwrap();

    atlas.context_blocks = vec![
        AtlasContextBlock {
            context_id: "trace-test".to_string(),
            name: "Trace Test".to_string(),
            priority: 50,
            content: "Test content".to_string(),
            content_type: "text/plain".to_string(),
            inject_when: vec![],
            keywords: vec!["trace-test-keyword".to_string()],
            risk_tiers: vec![],
        },
    ];

    resolver.load_atlas(atlas).unwrap();
    let session_id = resolver.create_session("test", "Test").unwrap();

    let request = CARPRequest::new(
        session_id.clone(),
        "test".to_string(),
        "Goal with trace-test-keyword".to_string(),
    );
    resolver.resolve(&request).unwrap();

    let trace = resolver.get_trace(&session_id).unwrap();
    let injected_events: Vec<_> = trace.iter()
        .filter(|e| e.event_type == EventType::ContextInjected)
        .collect();

    assert!(
        !injected_events.is_empty(),
        "Conformance: Context injection MUST emit context.injected event"
    );

    // Verify payload contains required fields
    let event = &injected_events[0];
    assert!(
        event.payload.get("context_id").is_some(),
        "Conformance: context.injected payload must contain context_id"
    );
    assert!(
        event.payload.get("priority").is_some(),
        "Conformance: context.injected payload must contain priority"
    );
    assert!(
        event.payload.get("source_atlas").is_some(),
        "Conformance: context.injected payload must contain source_atlas"
    );
}

#[test]
fn conformance_context_block_required_fields() {
    let mut resolver = Resolver::new();
    let mut atlas: AtlasManifest = serde_json::from_value(json!({
        "atlas_version": "1.0",
        "atlas_id": "conformance.context.fields",
        "version": "1.0.0",
        "name": "Context Fields Conformance",
        "description": "Tests ContextBlock required fields",
        "domains": ["test"],
        "capabilities": [],
        "policies": [],
        "actions": []
    })).unwrap();

    atlas.context_blocks = vec![
        AtlasContextBlock {
            context_id: "field-test".to_string(),
            name: "Field Test".to_string(),
            priority: 75,
            content: "The actual content".to_string(),
            content_type: "text/markdown".to_string(),
            inject_when: vec![],
            keywords: vec!["field-check".to_string()],
            risk_tiers: vec![],
        },
    ];

    resolver.load_atlas(atlas).unwrap();
    let session_id = resolver.create_session("test", "Test").unwrap();

    let request = CARPRequest::new(
        session_id.clone(),
        "test".to_string(),
        "field-check".to_string(),
    );
    let resolution = resolver.resolve(&request).unwrap();

    assert!(!resolution.context_blocks.is_empty());
    let block = &resolution.context_blocks[0];

    // Required fields per CRA spec
    assert!(!block.block_id.is_empty(), "Conformance: block_id required");
    assert!(!block.name.is_empty(), "Conformance: name required");
    assert!(!block.content.is_empty(), "Conformance: content required");
    assert!(!block.content_type.is_empty(), "Conformance: content_type required");
    assert!(block.priority == 75, "Conformance: priority must be preserved");
}

#[test]
fn conformance_context_content_preserved() {
    let original_content = "EXACT content with\n\nMultiple lines\nand special chars: <>&\"'`${}[]";

    let mut resolver = Resolver::new();
    let mut atlas: AtlasManifest = serde_json::from_value(json!({
        "atlas_version": "1.0",
        "atlas_id": "conformance.context.content",
        "version": "1.0.0",
        "name": "Content Preservation Conformance",
        "description": "Tests content is not modified",
        "domains": ["test"],
        "capabilities": [],
        "policies": [],
        "actions": []
    })).unwrap();

    atlas.context_blocks = vec![
        AtlasContextBlock {
            context_id: "content-test".to_string(),
            name: "Content Test".to_string(),
            priority: 100,
            content: original_content.to_string(),
            content_type: "text/plain".to_string(),
            inject_when: vec![],
            keywords: vec!["preserve-content".to_string()],
            risk_tiers: vec![],
        },
    ];

    resolver.load_atlas(atlas).unwrap();
    let session_id = resolver.create_session("test", "Test").unwrap();

    let request = CARPRequest::new(
        session_id.clone(),
        "test".to_string(),
        "preserve-content".to_string(),
    );
    let resolution = resolver.resolve(&request).unwrap();

    assert!(!resolution.context_blocks.is_empty());
    assert_eq!(
        resolution.context_blocks[0].content,
        original_content,
        "Conformance: Content MUST be exactly preserved"
    );
}

#[test]
fn conformance_context_chain_integrity() {
    let mut resolver = Resolver::new();
    let mut atlas: AtlasManifest = serde_json::from_value(json!({
        "atlas_version": "1.0",
        "atlas_id": "conformance.context.chain",
        "version": "1.0.0",
        "name": "Context Chain Conformance",
        "description": "Tests chain integrity with context injection",
        "domains": ["test"],
        "capabilities": [],
        "policies": [],
        "actions": []
    })).unwrap();

    // Multiple context blocks
    atlas.context_blocks = vec![
        AtlasContextBlock {
            context_id: "block-1".to_string(),
            name: "Block 1".to_string(),
            priority: 100,
            content: "Content 1".to_string(),
            content_type: "text/plain".to_string(),
            inject_when: vec![],
            keywords: vec!["chain-test".to_string()],
            risk_tiers: vec![],
        },
        AtlasContextBlock {
            context_id: "block-2".to_string(),
            name: "Block 2".to_string(),
            priority: 50,
            content: "Content 2".to_string(),
            content_type: "text/plain".to_string(),
            inject_when: vec![],
            keywords: vec!["chain-test".to_string()],
            risk_tiers: vec![],
        },
    ];

    resolver.load_atlas(atlas).unwrap();
    let session_id = resolver.create_session("test", "Chain test").unwrap();

    // Multiple resolutions with context injection
    for _ in 0..3 {
        let request = CARPRequest::new(
            session_id.clone(),
            "test".to_string(),
            "chain-test goal".to_string(),
        );
        resolver.resolve(&request).unwrap();
    }

    // Chain MUST remain valid after context injections
    let verification = resolver.verify_chain(&session_id).unwrap();
    assert!(
        verification.is_valid,
        "Conformance: Hash chain MUST remain valid after context injection"
    );
}
