//! CRA Client tests

use cra_wrapper::client::{CRAClient, DirectClient, BootstrapResult, ActionReport};

#[tokio::test]
async fn test_direct_client_bootstrap() {
    let client = DirectClient::new();

    let result = client.bootstrap("Help with coding").await.unwrap();

    // Should have a session ID
    assert!(!result.session_id.is_empty());

    // Should have genesis hash
    assert!(result.genesis_hash.starts_with("genesis_"));

    // Current hash should equal genesis hash
    assert_eq!(result.current_hash, result.genesis_hash);

    // Should have at least one rule
    assert!(!result.rules.is_empty());
    assert!(result.rules.iter().any(|r| r.rule_id == "trace.required"));
}

#[tokio::test]
async fn test_direct_client_request_context() {
    let client = DirectClient::new();

    let contexts = client.request_context(
        "session-123",
        "Need help with unit testing",
        Some(vec!["testing".to_string(), "rust".to_string()]),
    ).await.unwrap();

    // DirectClient returns empty contexts
    assert!(contexts.is_empty());
}

#[tokio::test]
async fn test_direct_client_report_action() {
    let client = DirectClient::new();

    let report = client.report_action(
        "session-123",
        "write_file",
        serde_json::json!({
            "path": "/tmp/test.txt",
            "content": "hello"
        }),
    ).await.unwrap();

    // DirectClient always approves
    assert_eq!(report.decision, "approved");
    assert!(!report.trace_id.is_empty());
    assert!(report.reason.is_none());
    assert!(!report.policy_notes.is_empty());
}

#[tokio::test]
async fn test_direct_client_feedback() {
    let client = DirectClient::new();

    // Feedback should succeed
    let result = client.feedback(
        "session-123",
        "ctx-456",
        true,
        Some("Very helpful context!"),
    ).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_direct_client_upload_trace() {
    let client = DirectClient::new();

    let events = vec![
        serde_json::json!({"event_type": "test.event", "data": 1}),
        serde_json::json!({"event_type": "test.event", "data": 2}),
        serde_json::json!({"event_type": "test.event", "data": 3}),
    ];

    let result = client.upload_trace(events).await.unwrap();

    assert_eq!(result.uploaded_count, 3);
    assert!(result.success);
}

#[tokio::test]
async fn test_direct_client_end_session() {
    let client = DirectClient::new();

    let result = client.end_session("session-123", Some("Task completed")).await.unwrap();

    assert!(result.chain_verified);
    assert!(result.final_hash.starts_with("final_"));
}

#[tokio::test]
async fn test_direct_client_default() {
    let client = DirectClient::default();

    let result = client.bootstrap("test").await.unwrap();
    assert!(!result.session_id.is_empty());
}

#[tokio::test]
async fn test_bootstrap_result_serialization() {
    use cra_wrapper::client::BootstrapContext;

    let result = BootstrapResult {
        session_id: "session-123".to_string(),
        genesis_hash: "genesis_abc".to_string(),
        current_hash: "genesis_abc".to_string(),
        context_ids: vec!["ctx-1".to_string()],
        contexts: vec![
            BootstrapContext {
                context_id: "ctx-1".to_string(),
                content: "You are a helpful assistant.".to_string(),
                priority: 100,
            }
        ],
        rules: vec![],
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("session-123"));

    let parsed: BootstrapResult = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.session_id, result.session_id);
}

#[tokio::test]
async fn test_action_report_serialization() {
    let report = ActionReport {
        decision: "approved".to_string(),
        trace_id: "trace-123".to_string(),
        reason: None,
        policy_notes: vec!["Permitted".to_string()],
    };

    let json = serde_json::to_string(&report).unwrap();
    let parsed: ActionReport = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.decision, report.decision);
    assert_eq!(parsed.trace_id, report.trace_id);
}

#[tokio::test]
async fn test_action_report_with_denial() {
    let report = ActionReport {
        decision: "denied".to_string(),
        trace_id: "trace-123".to_string(),
        reason: Some("Action not permitted by policy".to_string()),
        policy_notes: vec!["Blocked by security policy".to_string()],
    };

    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("denied"));
    assert!(json.contains("Action not permitted"));

    let parsed: ActionReport = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.reason, Some("Action not permitted by policy".to_string()));
}
