//! Bootstrap protocol tests

use cra_mcp::bootstrap::*;
use cra_mcp::session::Session;

#[test]
fn test_bootstrap_state_machine() {
    let protocol = BootstrapProtocol::new();

    // Initial state should be AwaitingInit
    assert_eq!(protocol.state(), BootstrapState::AwaitingInit);
    assert!(!protocol.is_complete());
}

#[test]
fn test_handle_init_creates_governance() {
    let mut protocol = BootstrapProtocol::new();

    let init = InitMessage {
        agent_id: "test-agent".to_string(),
        capabilities: AgentCapabilities {
            tools: vec!["read".to_string(), "write".to_string()],
            protocols: vec!["mcp".to_string()],
            context_window: Some(100000),
            extra: std::collections::HashMap::new(),
        },
        intent: "Help user with coding tasks".to_string(),
    };

    let session = Session::new(
        "test-agent".to_string(),
        "Help user".to_string(),
        vec!["code-atlas".to_string()],
        "genesis_hash_123".to_string(),
    );

    let governance = protocol.handle_init(init, session).unwrap();

    // Should transition to AwaitingAck
    assert_eq!(protocol.state(), BootstrapState::AwaitingAck);

    // Should have standard rules
    assert!(!governance.rules.is_empty());
    assert!(governance.rules.iter().any(|r| r.rule_id == "trace.required"));
    assert!(governance.acknowledgment_required);
}

#[test]
fn test_init_wrong_state_fails() {
    let mut protocol = BootstrapProtocol::new();

    // First INIT
    let session = Session::new(
        "agent".to_string(),
        "goal".to_string(),
        vec![],
        "genesis".to_string(),
    );

    let init = InitMessage {
        agent_id: "agent".to_string(),
        capabilities: AgentCapabilities {
            tools: vec![],
            protocols: vec![],
            context_window: None,
            extra: std::collections::HashMap::new(),
        },
        intent: "test".to_string(),
    };

    protocol.handle_init(init.clone(), session.clone()).unwrap();

    // Second INIT should fail (wrong state)
    let result = protocol.handle_init(init, session);
    assert!(result.is_err());
}

#[test]
fn test_handle_ack_transitions_to_streaming() {
    let mut protocol = BootstrapProtocol::new();

    // Set up protocol through INIT
    let session = Session::new(
        "agent".to_string(),
        "goal".to_string(),
        vec![],
        "genesis".to_string(),
    );

    let init = InitMessage {
        agent_id: "agent".to_string(),
        capabilities: AgentCapabilities {
            tools: vec![],
            protocols: vec![],
            context_window: None,
            extra: std::collections::HashMap::new(),
        },
        intent: "test".to_string(),
    };

    let governance = protocol.handle_init(init, session).unwrap();

    // Create ACK message acknowledging all rules
    let ack = AckMessage {
        session_id: governance.session_id.clone(),
        previous_hash: governance.genesis_hash.clone(),
        acknowledgments: governance.rules.iter().map(|r| RuleAcknowledgment {
            rule_id: r.rule_id.clone(),
            understood: true,
        }).collect(),
        wrapper_state: "building".to_string(),
    };

    protocol.handle_ack(ack).unwrap();

    // Should transition to StreamingContext
    assert_eq!(protocol.state(), BootstrapState::StreamingContext);
}

#[test]
fn test_ack_missing_hard_rule_fails() {
    let mut protocol = BootstrapProtocol::new();

    let session = Session::new(
        "agent".to_string(),
        "goal".to_string(),
        vec![],
        "genesis".to_string(),
    );

    let init = InitMessage {
        agent_id: "agent".to_string(),
        capabilities: AgentCapabilities {
            tools: vec![],
            protocols: vec![],
            context_window: None,
            extra: std::collections::HashMap::new(),
        },
        intent: "test".to_string(),
    };

    let governance = protocol.handle_init(init, session).unwrap();

    // ACK without acknowledging any rules
    let ack = AckMessage {
        session_id: governance.session_id.clone(),
        previous_hash: governance.genesis_hash.clone(),
        acknowledgments: vec![], // Missing acknowledgments!
        wrapper_state: "building".to_string(),
    };

    let result = protocol.handle_ack(ack);
    assert!(result.is_err());
}

#[test]
fn test_generate_context_message() {
    let mut protocol = BootstrapProtocol::new();

    // Set up to StreamingContext state
    let session = Session::new(
        "agent".to_string(),
        "goal".to_string(),
        vec![],
        "genesis_hash".to_string(),
    );

    let init = InitMessage {
        agent_id: "agent".to_string(),
        capabilities: AgentCapabilities {
            tools: vec![],
            protocols: vec![],
            context_window: None,
            extra: std::collections::HashMap::new(),
        },
        intent: "test".to_string(),
    };

    let governance = protocol.handle_init(init, session).unwrap();

    let ack = AckMessage {
        session_id: governance.session_id.clone(),
        previous_hash: governance.genesis_hash.clone(),
        acknowledgments: governance.rules.iter().map(|r| RuleAcknowledgment {
            rule_id: r.rule_id.clone(),
            understood: true,
        }).collect(),
        wrapper_state: "building".to_string(),
    };

    protocol.handle_ack(ack).unwrap();

    // Generate context message
    let contexts = vec![
        BootstrapContext {
            context_id: "ctx-1".to_string(),
            priority: 100,
            inject_mode: "system".to_string(),
            content: "You are a helpful assistant.".to_string(),
            digest: "abc123".to_string(),
        },
    ];

    let context_msg = protocol.generate_context(contexts, false).unwrap();

    assert_eq!(context_msg.sequence, 0);
    assert!(!context_msg.more_available);
    assert_eq!(context_msg.contexts.len(), 1);

    // Should transition to AwaitingReady
    assert_eq!(protocol.state(), BootstrapState::AwaitingReady);
}

#[test]
fn test_handle_ready_completes_bootstrap() {
    let mut protocol = BootstrapProtocol::new();

    // Go through full state machine
    let session = Session::new(
        "agent".to_string(),
        "goal".to_string(),
        vec![],
        "genesis_hash".to_string(),
    );

    let init = InitMessage {
        agent_id: "agent".to_string(),
        capabilities: AgentCapabilities {
            tools: vec![],
            protocols: vec![],
            context_window: None,
            extra: std::collections::HashMap::new(),
        },
        intent: "test".to_string(),
    };

    let governance = protocol.handle_init(init, session).unwrap();
    let session_id = governance.session_id.clone();

    let ack = AckMessage {
        session_id: session_id.clone(),
        previous_hash: governance.genesis_hash.clone(),
        acknowledgments: governance.rules.iter().map(|r| RuleAcknowledgment {
            rule_id: r.rule_id.clone(),
            understood: true,
        }).collect(),
        wrapper_state: "building".to_string(),
    };

    protocol.handle_ack(ack).unwrap();

    let context_msg = protocol.generate_context(vec![], false).unwrap();

    // Send READY
    let ready = ReadyMessage {
        session_id: session_id.clone(),
        previous_hash: context_msg.previous_hash.clone(),
        wrapper_state: "complete".to_string(),
        internalized_contexts: vec![],
        ready_for: "work".to_string(),
    };

    let session_msg = protocol.handle_ready(ready).unwrap();

    // Should be complete
    assert!(protocol.is_complete());
    assert_eq!(session_msg.status, "active");
    assert!(!session_msg.tools_available.is_empty());
}

#[test]
fn test_ready_incomplete_wrapper_fails() {
    let mut protocol = BootstrapProtocol::new();

    // Set up to AwaitingReady state
    let session = Session::new(
        "agent".to_string(),
        "goal".to_string(),
        vec![],
        "genesis_hash".to_string(),
    );

    let init = InitMessage {
        agent_id: "agent".to_string(),
        capabilities: AgentCapabilities {
            tools: vec![],
            protocols: vec![],
            context_window: None,
            extra: std::collections::HashMap::new(),
        },
        intent: "test".to_string(),
    };

    let governance = protocol.handle_init(init, session).unwrap();
    let session_id = governance.session_id.clone();

    let ack = AckMessage {
        session_id: session_id.clone(),
        previous_hash: governance.genesis_hash.clone(),
        acknowledgments: governance.rules.iter().map(|r| RuleAcknowledgment {
            rule_id: r.rule_id.clone(),
            understood: true,
        }).collect(),
        wrapper_state: "building".to_string(),
    };

    protocol.handle_ack(ack).unwrap();
    protocol.generate_context(vec![], false).unwrap();

    // READY with incomplete wrapper
    let ready = ReadyMessage {
        session_id: session_id.clone(),
        previous_hash: "hash".to_string(),
        wrapper_state: "building".to_string(), // Not complete!
        internalized_contexts: vec![],
        ready_for: "work".to_string(),
    };

    let result = protocol.handle_ready(ready);
    assert!(result.is_err());
}

#[test]
fn test_bootstrap_message_serialization() {
    let init = InitMessage {
        agent_id: "test".to_string(),
        capabilities: AgentCapabilities {
            tools: vec!["read".to_string()],
            protocols: vec!["mcp".to_string()],
            context_window: Some(100000),
            extra: std::collections::HashMap::new(),
        },
        intent: "test intent".to_string(),
    };

    let message = BootstrapMessage::Init(init);
    let json = serde_json::to_string(&message).unwrap();

    // Should serialize with "type": "INIT"
    assert!(json.contains("\"type\":\"INIT\""));

    // Should deserialize back
    let parsed: BootstrapMessage = serde_json::from_str(&json).unwrap();
    match parsed {
        BootstrapMessage::Init(i) => {
            assert_eq!(i.agent_id, "test");
        }
        _ => panic!("Wrong message type"),
    }
}
