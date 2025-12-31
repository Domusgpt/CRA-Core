//! Session management tests

use cra_mcp::session::{Session, SessionManager};

#[test]
fn test_session_creation() {
    let session = Session::new(
        "agent-123".to_string(),
        "Help with code".to_string(),
        vec!["code-atlas".to_string()],
        "genesis_abc123".to_string(),
    );

    assert!(!session.session_id.is_empty());
    assert_eq!(session.agent_id, "agent-123");
    assert_eq!(session.goal, "Help with code");
    assert_eq!(session.genesis_hash, "genesis_abc123");
    assert_eq!(session.current_hash, "genesis_abc123");
    assert_eq!(session.event_count, 1); // Genesis event
    assert!(session.injected_contexts.is_empty());
}

#[test]
fn test_session_update_hash() {
    let mut session = Session::new(
        "agent".to_string(),
        "goal".to_string(),
        vec![],
        "genesis".to_string(),
    );

    assert_eq!(session.event_count, 1);

    session.update_hash("hash_2".to_string());

    assert_eq!(session.current_hash, "hash_2");
    assert_eq!(session.event_count, 2);

    session.update_hash("hash_3".to_string());

    assert_eq!(session.current_hash, "hash_3");
    assert_eq!(session.event_count, 3);
}

#[test]
fn test_session_record_context_injection() {
    let mut session = Session::new(
        "agent".to_string(),
        "goal".to_string(),
        vec![],
        "genesis".to_string(),
    );

    assert!(session.injected_contexts.is_empty());

    session.record_context_injection("ctx-1".to_string());
    session.record_context_injection("ctx-2".to_string());

    assert_eq!(session.injected_contexts.len(), 2);
    assert!(session.injected_contexts.contains(&"ctx-1".to_string()));

    // Duplicate should not be added
    session.record_context_injection("ctx-1".to_string());
    assert_eq!(session.injected_contexts.len(), 2);
}

#[test]
fn test_session_duration() {
    let session = Session::new(
        "agent".to_string(),
        "goal".to_string(),
        vec![],
        "genesis".to_string(),
    );

    // Duration should be very small (just created)
    let duration = session.duration_ms();
    assert!(duration >= 0);
    assert!(duration < 1000); // Less than 1 second
}

#[test]
fn test_session_manager_creation() {
    let manager = SessionManager::new();

    let atlases = manager.list_atlases().unwrap();
    assert!(atlases.is_empty()); // No atlases loaded by default
}

#[test]
fn test_session_manager_start_session() {
    let manager = SessionManager::new();

    let session = manager.start_session(
        "test-agent".to_string(),
        "Help with coding".to_string(),
        None,
    ).unwrap();

    assert!(!session.session_id.is_empty());
    assert_eq!(session.agent_id, "test-agent");
    assert_eq!(session.goal, "Help with coding");

    // Should be retrievable
    let retrieved = manager.get_session(&session.session_id).unwrap();
    assert_eq!(retrieved.session_id, session.session_id);
}

#[test]
fn test_session_manager_get_current_session() {
    let manager = SessionManager::new();

    // No session should be current initially
    let result = manager.get_current_session();
    assert!(result.is_err());

    // Start a session
    let session1 = manager.start_session(
        "agent-1".to_string(),
        "goal 1".to_string(),
        None,
    ).unwrap();

    let current = manager.get_current_session().unwrap();
    assert_eq!(current.session_id, session1.session_id);

    // Start another session (should become current)
    std::thread::sleep(std::time::Duration::from_millis(10));
    let session2 = manager.start_session(
        "agent-2".to_string(),
        "goal 2".to_string(),
        None,
    ).unwrap();

    let current = manager.get_current_session().unwrap();
    assert_eq!(current.session_id, session2.session_id);
}

#[test]
fn test_session_manager_end_session() {
    let manager = SessionManager::new();

    let session = manager.start_session(
        "agent".to_string(),
        "goal".to_string(),
        None,
    ).unwrap();

    let session_id = session.session_id.clone();

    // End the session
    let ended = manager.end_session(&session_id, Some("Task complete".to_string())).unwrap();
    assert_eq!(ended.session_id, session_id);

    // Session should no longer be retrievable
    let result = manager.get_session(&session_id);
    assert!(result.is_err());
}

#[test]
fn test_session_manager_report_action() {
    let manager = SessionManager::new();

    let session = manager.start_session(
        "agent".to_string(),
        "goal".to_string(),
        None,
    ).unwrap();

    // Report an action
    let report = manager.report_action(
        &session.session_id,
        "write_file",
        serde_json::json!({"path": "/tmp/test.txt"}),
    ).unwrap();

    // Default behavior should approve (no policies loaded)
    assert_eq!(report.decision, "approved");
    assert!(!report.trace_id.is_empty());
}

#[test]
fn test_session_manager_get_trace() {
    let manager = SessionManager::new();

    let session = manager.start_session(
        "agent".to_string(),
        "goal".to_string(),
        None,
    ).unwrap();

    // Get trace for the session
    let trace = manager.get_trace(&session.session_id).unwrap();

    // Should have at least the genesis event
    assert!(!trace.is_empty());
}

#[test]
fn test_session_manager_verify_chain() {
    let manager = SessionManager::new();

    let session = manager.start_session(
        "agent".to_string(),
        "goal".to_string(),
        None,
    ).unwrap();

    // Verify the chain
    let verification = manager.verify_chain(&session.session_id).unwrap();

    // Should be valid (no tampering)
    assert!(verification.is_valid);
}

#[test]
fn test_session_manager_invalid_session() {
    let manager = SessionManager::new();

    // Try to get non-existent session
    let result = manager.get_session("non-existent-session");
    assert!(result.is_err());

    // Try to end non-existent session
    let result = manager.end_session("non-existent-session", None);
    assert!(result.is_err());
}

#[test]
fn test_session_serialization() {
    let session = Session::new(
        "agent-123".to_string(),
        "Test goal".to_string(),
        vec!["atlas-1".to_string()],
        "genesis_hash".to_string(),
    );

    // Should serialize to JSON
    let json = serde_json::to_string(&session).unwrap();
    assert!(json.contains("agent-123"));
    assert!(json.contains("Test goal"));

    // Should deserialize back
    let parsed: Session = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.agent_id, session.agent_id);
    assert_eq!(parsed.goal, session.goal);
}
