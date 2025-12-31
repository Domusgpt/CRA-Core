//! Wrapper integration tests

use cra_wrapper::{Wrapper, WrapperConfig, WrapperSession, ProcessedInput, ProcessedOutput};

#[tokio::test]
async fn test_wrapper_creation() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    // No session should be active
    let session = wrapper.current_session().await;
    assert!(session.is_none());
}

#[tokio::test]
async fn test_wrapper_start_session() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    let session_id = wrapper.start_session("Help with coding").await.unwrap();

    assert!(!session_id.is_empty());

    // Session should be active
    let session = wrapper.current_session().await;
    assert!(session.is_some());

    let session = session.unwrap();
    assert_eq!(session.session_id, session_id);
    assert_eq!(session.goal, "Help with coding");
}

#[tokio::test]
async fn test_wrapper_end_session() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    // Start session
    let session_id = wrapper.start_session("Test goal").await.unwrap();

    // End session
    let summary = wrapper.end_session(Some("Task complete")).await.unwrap();

    assert_eq!(summary.session_id, session_id);
    assert!(summary.chain_verified);
    assert!(!summary.final_hash.is_empty());

    // Session should be inactive
    assert!(wrapper.current_session().await.is_none());
}

#[tokio::test]
async fn test_wrapper_on_input() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    wrapper.start_session("Test goal").await.unwrap();

    let result = wrapper.on_input("Hello, can you help me?").await.unwrap();

    assert_eq!(result.original, "Hello, can you help me?");
    assert_eq!(result.processed, "Hello, can you help me?");
}

#[tokio::test]
async fn test_wrapper_on_output() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    wrapper.start_session("Test goal").await.unwrap();

    let result = wrapper.on_output("Sure, I can help!").await.unwrap();

    assert_eq!(result.original, "Sure, I can help!");
    assert_eq!(result.processed, "Sure, I can help!");
}

#[tokio::test]
async fn test_wrapper_report_action() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    wrapper.start_session("Test goal").await.unwrap();

    let decision = wrapper.report_action(
        "write_file",
        serde_json::json!({
            "path": "/tmp/test.txt",
            "content": "hello world"
        }),
    ).await.unwrap();

    // DirectClient always approves
    assert!(decision.allowed);
}

#[tokio::test]
async fn test_wrapper_feedback() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    wrapper.start_session("Test goal").await.unwrap();

    // Submit feedback
    let result = wrapper.feedback("ctx-1", true, Some("Very helpful!")).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_wrapper_request_context() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    wrapper.start_session("Test goal").await.unwrap();

    let contexts = wrapper.request_context(
        "How to write unit tests",
        Some(vec!["testing".to_string(), "rust".to_string()]),
    ).await.unwrap();

    // DirectClient returns empty contexts
    assert!(contexts.is_empty());
}

#[tokio::test]
async fn test_wrapper_no_session_errors() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    // All operations should fail without a session

    let result = wrapper.on_input("test").await;
    assert!(result.is_err());

    let result = wrapper.on_output("test").await;
    assert!(result.is_err());

    let result = wrapper.report_action("test", serde_json::json!({})).await;
    assert!(result.is_err());

    let result = wrapper.end_session(None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_wrapper_queue_stats() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    wrapper.start_session("Test goal").await.unwrap();

    // Do some operations that generate events
    wrapper.on_input("input 1").await.unwrap();
    wrapper.on_output("output 1").await.unwrap();

    let stats = wrapper.queue_stats().await;

    // Session start + input + output events
    assert!(stats.total_enqueued >= 3);
}

#[tokio::test]
async fn test_wrapper_cache_stats() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    wrapper.start_session("Test goal").await.unwrap();

    let stats = wrapper.cache_stats().await;

    // Should have cached initial contexts from bootstrap
    assert_eq!(stats.entry_count, 0); // DirectClient provides no contexts
}

#[tokio::test]
async fn test_wrapper_session_serialization() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    wrapper.start_session("Serialization test").await.unwrap();

    let session = wrapper.current_session().await.unwrap();

    // Should serialize to JSON
    let json = serde_json::to_string(&session).unwrap();
    assert!(json.contains("Serialization test"));

    // Should deserialize back
    let parsed: WrapperSession = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.goal, session.goal);
}

#[tokio::test]
async fn test_wrapper_multiple_sessions() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    // Start first session
    let session1_id = wrapper.start_session("Goal 1").await.unwrap();
    wrapper.end_session(Some("Done")).await.unwrap();

    // Start second session
    let session2_id = wrapper.start_session("Goal 2").await.unwrap();

    // IDs should be different
    assert_ne!(session1_id, session2_id);

    let current = wrapper.current_session().await.unwrap();
    assert_eq!(current.goal, "Goal 2");
}

#[tokio::test]
async fn test_wrapper_with_default_config() {
    let config = WrapperConfig::default();
    let wrapper = Wrapper::new(config);

    let session_id = wrapper.start_session("Default config test").await.unwrap();
    assert!(!session_id.is_empty());
}

#[tokio::test]
async fn test_processed_input_structure() {
    let input = ProcessedInput {
        original: "original text".to_string(),
        processed: "processed text".to_string(),
        injected_context: vec!["context 1".to_string()],
    };

    let json = serde_json::to_string(&input).unwrap();
    let parsed: ProcessedInput = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.original, input.original);
    assert_eq!(parsed.processed, input.processed);
    assert_eq!(parsed.injected_context, input.injected_context);
}

#[tokio::test]
async fn test_processed_output_structure() {
    let output = ProcessedOutput {
        original: "original".to_string(),
        processed: "processed".to_string(),
    };

    let json = serde_json::to_string(&output).unwrap();
    let parsed: ProcessedOutput = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.original, output.original);
    assert_eq!(parsed.processed, output.processed);
}

#[tokio::test]
async fn test_wrapper_config_serialization() {
    let config = WrapperConfig::default();

    let json = serde_json::to_string(&config).unwrap();
    let parsed: WrapperConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.version, config.version);
    assert_eq!(parsed.checkpoints_enabled, config.checkpoints_enabled);
}
