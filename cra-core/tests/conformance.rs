//! Conformance tests against golden traces from specs/conformance/
//!
//! These tests verify that the Rust implementation matches the protocol specification
//! by comparing output against known-good golden traces.

use cra_core::atlas::AtlasManifest;
use cra_core::carp::{CARPRequest, Decision, Resolver};
use serde_json::{json, Value};

/// Load the simple-resolve test atlas
fn load_simple_atlas() -> AtlasManifest {
    let atlas_json = include_str!("../../specs/conformance/golden/simple-resolve/atlas.json");
    serde_json::from_str(atlas_json).expect("Failed to parse atlas.json")
}

/// Load the expected resolution
fn load_expected_resolution() -> Value {
    let resolution_json =
        include_str!("../../specs/conformance/golden/simple-resolve/expected-resolution.json");
    serde_json::from_str(resolution_json).expect("Failed to parse expected-resolution.json")
}

/// Load the expected trace events
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
    let expected = load_expected_resolution();

    // Check decision type
    assert_eq!(
        resolution.decision,
        Decision::Partial,
        "Decision should be 'partial' when some actions are denied"
    );

    // Check allowed actions
    let allowed_ids: Vec<&str> = resolution
        .allowed_actions
        .iter()
        .map(|a| a.action_id.as_str())
        .collect();
    assert!(
        allowed_ids.contains(&"data.get"),
        "data.get should be allowed"
    );
    assert!(
        allowed_ids.contains(&"data.list"),
        "data.list should be allowed"
    );
    assert_eq!(allowed_ids.len(), 2, "Should have exactly 2 allowed actions");

    // Check denied actions
    let denied_ids: Vec<&str> = resolution
        .denied_actions
        .iter()
        .map(|a| a.action_id.as_str())
        .collect();
    assert!(
        denied_ids.contains(&"data.delete"),
        "data.delete should be denied"
    );
    assert_eq!(denied_ids.len(), 1, "Should have exactly 1 denied action");

    // Check deny reason
    let delete_denial = resolution
        .denied_actions
        .iter()
        .find(|a| a.action_id == "data.delete")
        .expect("data.delete denial not found");
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

    // Load expected trace
    let expected_events = load_expected_trace();

    // Verify event types match (order matters)
    let actual_event_types: Vec<String> = trace
        .iter()
        .map(|e| e.event_type.to_string())
        .collect();

    let expected_event_types: Vec<String> = expected_events
        .iter()
        .map(|e| e["event_type"].as_str().unwrap().to_string())
        .collect();

    assert_eq!(
        actual_event_types.len(),
        expected_event_types.len(),
        "Event count mismatch: got {:?}, expected {:?}",
        actual_event_types,
        expected_event_types
    );

    for (i, (actual, expected)) in actual_event_types
        .iter()
        .zip(expected_event_types.iter())
        .enumerate()
    {
        assert_eq!(
            actual, expected,
            "Event type mismatch at index {}: got '{}', expected '{}'",
            i, actual, expected
        );
    }
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
    let expected_events = load_expected_trace();

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
    assert_eq!(carp_resolution.payload["allowed_count"].as_i64().unwrap(), 2);
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
