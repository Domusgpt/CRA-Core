//! VIB3+ Detailed Execution Trace
//!
//! This test creates a DETAILED step-by-step comparison showing exactly
//! what an agent sees and does with and without CRA context injection.

use cra_core::{
    Resolver, CARPRequest,
    atlas::AtlasManifest,
    trace::EventType,
};
use serde_json::json;

/// Simulate what an agent WITHOUT context would think/do
fn simulate_agent_without_context(goal: &str) -> Vec<String> {
    vec![
        format!("📥 RECEIVED GOAL: \"{}\"", goal),
        "🧠 THINKING: User wants a 4D visualization...".to_string(),
        "🧠 THINKING: '4D polychora' sounds like complex geometry".to_string(),
        "🧠 THINKING: I should look for a polychora rendering system".to_string(),
        "".to_string(),
        "⚡ ACTION: Search for 'polychora WebGL rendering'".to_string(),
        "⚡ ACTION: Find VIB3+ has a 'polychora' system option".to_string(),
        "⚡ ACTION: Attempt to use system=polychora".to_string(),
        "".to_string(),
        "❌ RESULT: Polychora system renders nothing (it's a placeholder!)".to_string(),
        "❌ RESULT: User sees blank screen".to_string(),
        "❌ RESULT: Agent confused, tries to debug non-functional code".to_string(),
        "".to_string(),
        "🔄 RETRY: Agent tries building 4D projection from scratch".to_string(),
        "🔄 RETRY: Implements complex tesseract rotation matrices".to_string(),
        "🔄 RETRY: Spends significant time on approach that won't integrate".to_string(),
        "".to_string(),
        "💀 FINAL: Hours wasted, user frustrated, no working result".to_string(),
    ]
}

/// Simulate what an agent WITH context would think/do
fn simulate_agent_with_context(goal: &str, context_blocks: &[String]) -> Vec<String> {
    let mut steps = vec![
        format!("📥 RECEIVED GOAL: \"{}\"", goal),
        "".to_string(),
        "📚 CRA CONTEXT INJECTED:".to_string(),
    ];

    for block in context_blocks {
        steps.push(format!("   └─ {}", block));
    }

    steps.extend(vec![
        "".to_string(),
        "🧠 THINKING: User wants a 4D visualization...".to_string(),
        "🧠 THINKING: Wait - context says 'polychora is PLACEHOLDER ONLY'!".to_string(),
        "🧠 THINKING: Context says working systems are: faceted, quantum, holographic".to_string(),
        "🧠 THINKING: For '4D effect', holographic with complex geometry would work".to_string(),
        "".to_string(),
        "⚡ ACTION: Use geometry formula from context: coreIndex * 8 + baseIndex".to_string(),
        "⚡ ACTION: For torus (base=3) with core=2: geometry = 2*8+3 = 19".to_string(),
        "⚡ ACTION: Use iframe embed pattern from context".to_string(),
        "".to_string(),
        "✅ CODE GENERATED:".to_string(),
        "   ```html".to_string(),
        "   <div style=\"position:fixed;inset:0;z-index:-1\">".to_string(),
        "     <iframe src=\"https://domusgpt.github.io/vib3-plus-engine/".to_string(),
        "       ?system=holographic&geometry=19&hue=280\"".to_string(),
        "       style=\"width:100%;height:100%;border:none\">".to_string(),
        "     </iframe>".to_string(),
        "   </div>".to_string(),
        "   ```".to_string(),
        "".to_string(),
        "✅ RESULT: Working animated background on first attempt!".to_string(),
        "✅ RESULT: User sees beautiful holographic torus effect".to_string(),
        "✅ RESULT: Task completed in minutes, not hours".to_string(),
    ]);

    steps
}

fn create_no_context_atlas() -> AtlasManifest {
    serde_json::from_value(json!({
        "atlas_version": "1.0",
        "atlas_id": "demo.no-vib3",
        "version": "1.0.0",
        "name": "No VIB3 Context Atlas",
        "description": "Atlas with NO VIB3 context",
        "domains": ["webgl"],
        "capabilities": [],
        "policies": [],
        "actions": [{
            "action_id": "code.write",
            "name": "Write Code",
            "description": "Write source code",
            "parameters_schema": {"type": "object"},
            "risk_tier": "low"
        }]
    })).unwrap()
}

fn load_vib3_atlas() -> AtlasManifest {
    let atlas_json = include_str!("../../atlases/vib3-webpage-development.json");
    serde_json::from_str(atlas_json).expect("Failed to parse vib3-webpage-development.json")
}

#[test]
fn detailed_execution_comparison() {
    let goal = "Add a 4D polychora visualization to the webpage background";

    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║                    DETAILED EXECUTION COMPARISON                           ║");
    println!("║            Agent WITHOUT CRA vs Agent WITH CRA Context                     ║");
    println!("╚════════════════════════════════════════════════════════════════════════════╝");
    println!("\n🎯 USER REQUEST: \"{}\"", goal);

    // ═══════════════════════════════════════════════════════════════════════════
    // AGENT A: WITHOUT CRA CONTEXT
    // ═══════════════════════════════════════════════════════════════════════════

    println!("\n");
    println!("┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓");
    println!("┃                    AGENT A: WITHOUT CRA CONTEXT                            ┃");
    println!("┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛");

    let mut resolver_a = Resolver::new();
    resolver_a.load_atlas(create_no_context_atlas()).unwrap();
    let session_a = resolver_a.create_session("agent-a", "no context").unwrap();

    let request_a = CARPRequest::new(
        session_a.clone(),
        "agent-a".to_string(),
        goal.to_string(),
    );

    let resolution_a = resolver_a.resolve(&request_a).unwrap();

    println!("\n┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 1: CRA RESOLUTION                                                      │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    println!("│ Decision: {:?}", resolution_a.decision);
    println!("│ Allowed actions: {}", resolution_a.allowed_actions.len());
    println!("│ Context blocks injected: {} ⚠️  NONE!", resolution_a.context_blocks.len());
    println!("└─────────────────────────────────────────────────────────────────────────────┘");

    println!("\n┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 2: WHAT THE AGENT SEES                                                 │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    println!("│ Goal: \"{}\"", goal);
    println!("│ Context: (empty)");
    println!("│ ");
    println!("│ The agent must figure out VIB3+ entirely on its own.");
    println!("│ No warnings about broken systems. No embed patterns. No geometry formula.");
    println!("└─────────────────────────────────────────────────────────────────────────────┘");

    println!("\n┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 3: SIMULATED AGENT EXECUTION                                           │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    for step in simulate_agent_without_context(goal) {
        if step.is_empty() {
            println!("│");
        } else {
            println!("│ {}", step);
        }
    }
    println!("└─────────────────────────────────────────────────────────────────────────────┘");

    // Show TRACE events
    let trace_a = resolver_a.get_trace(&session_a).unwrap();
    println!("\n┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 4: TRACE AUDIT LOG                                                     │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    for event in &trace_a {
        println!("│ [seq {}] {:<30} hash: {}...",
            event.sequence,
            event.event_type.to_string(),
            &event.event_hash[..12]
        );
    }
    println!("│");
    println!("│ Note: NO context.injected events - agent received no guidance");
    println!("└─────────────────────────────────────────────────────────────────────────────┘");

    // ═══════════════════════════════════════════════════════════════════════════
    // AGENT B: WITH CRA CONTEXT
    // ═══════════════════════════════════════════════════════════════════════════

    println!("\n");
    println!("┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓");
    println!("┃                     AGENT B: WITH CRA CONTEXT                              ┃");
    println!("┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛");

    let mut resolver_b = Resolver::new();
    let atlas_b = load_vib3_atlas();
    println!("\n📦 Atlas loaded: {} v{}", atlas_b.name, atlas_b.version);
    println!("   {} context blocks available", atlas_b.context_blocks.len());

    resolver_b.load_atlas(atlas_b).unwrap();
    let session_b = resolver_b.create_session("agent-b", "with context").unwrap();

    let request_b = CARPRequest::new(
        session_b.clone(),
        "agent-b".to_string(),
        goal.to_string(),
    );

    let resolution_b = resolver_b.resolve(&request_b).unwrap();

    println!("\n┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 1: CRA RESOLUTION                                                      │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    println!("│ Decision: {:?}", resolution_b.decision);
    println!("│ Allowed actions: {}", resolution_b.allowed_actions.len());
    println!("│ Context blocks injected: {} ✅ GUIDANCE PROVIDED!", resolution_b.context_blocks.len());
    println!("└─────────────────────────────────────────────────────────────────────────────┘");

    println!("\n┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 2: WHAT THE AGENT SEES                                                 │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    println!("│ Goal: \"{}\"", goal);
    println!("│");
    println!("│ INJECTED CONTEXT:");

    let mut context_names = Vec::new();
    for block in &resolution_b.context_blocks {
        context_names.push(block.block_id.clone());
        println!("│");
        println!("│ ╭─── {} (priority {}) ───╮", block.block_id, block.priority);
        for (i, line) in block.content.lines().enumerate() {
            if i < 20 {
                println!("│ │ {}", line);
            }
        }
        if block.content.lines().count() > 20 {
            println!("│ │ ... [{} more lines]", block.content.lines().count() - 20);
        }
        println!("│ ╰─────────────────────────────────────────────────────────────────────╯");
    }
    println!("└─────────────────────────────────────────────────────────────────────────────┘");

    println!("\n┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 3: SIMULATED AGENT EXECUTION                                           │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    for step in simulate_agent_with_context(goal, &context_names) {
        if step.is_empty() {
            println!("│");
        } else {
            println!("│ {}", step);
        }
    }
    println!("└─────────────────────────────────────────────────────────────────────────────┘");

    // Show TRACE events
    let trace_b = resolver_b.get_trace(&session_b).unwrap();
    println!("\n┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 4: TRACE AUDIT LOG                                                     │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    for event in &trace_b {
        let marker = if event.event_type == EventType::ContextInjected { "⭐" } else { "  " };
        println!("│ {} [seq {}] {:<30} hash: {}...",
            marker,
            event.sequence,
            event.event_type.to_string(),
            &event.event_hash[..12]
        );

        if event.event_type == EventType::ContextInjected {
            if let Some(ctx_id) = event.payload.get("context_id") {
                println!("│         └─ Context: {}", ctx_id);
            }
        }
    }
    println!("│");
    println!("│ ⭐ = context.injected event (cryptographic proof of guidance)");
    println!("└─────────────────────────────────────────────────────────────────────────────┘");

    // Verify chains
    let verify_a = resolver_a.verify_chain(&session_a).unwrap();
    let verify_b = resolver_b.verify_chain(&session_b).unwrap();

    // ═══════════════════════════════════════════════════════════════════════════
    // FINAL COMPARISON
    // ═══════════════════════════════════════════════════════════════════════════

    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║                         FINAL COMPARISON                                   ║");
    println!("╠════════════════════════════════════════════════════════════════════════════╣");
    println!("║                                                                            ║");
    println!("║  Metric                      │ Without CRA    │ With CRA                   ║");
    println!("║  ────────────────────────────┼────────────────┼──────────────────────────  ║");
    println!("║  Context blocks              │ 0              │ {}                          ║", resolution_b.context_blocks.len());
    println!("║  TRACE events                │ {}              │ {}                          ║", trace_a.len(), trace_b.len());
    println!("║  context.injected events     │ 0              │ {}                          ║",
        trace_b.iter().filter(|e| e.event_type == EventType::ContextInjected).count());
    println!("║  Chain integrity             │ {}            │ {}                        ║",
        if verify_a.is_valid { "✅ Valid" } else { "❌ Invalid" },
        if verify_b.is_valid { "✅ Valid" } else { "❌ Invalid" });
    println!("║                                                                            ║");
    println!("║  Agent behavior              │ Tries broken   │ Uses working system        ║");
    println!("║                              │ 'polychora'    │ (holographic)              ║");
    println!("║                                                                            ║");
    println!("║  Time to solution            │ Hours (fail)   │ Minutes (success)          ║");
    println!("║                                                                            ║");
    println!("║  Auditability                │ No proof of    │ Cryptographic proof        ║");
    println!("║                              │ what agent saw │ of injected context        ║");
    println!("║                                                                            ║");
    println!("╚════════════════════════════════════════════════════════════════════════════╝");

    println!("\n");
    println!("🔑 KEY INSIGHT:");
    println!("   CRA's context.injected events provide CRYPTOGRAPHIC PROOF");
    println!("   of exactly what guidance the agent received.");
    println!("");
    println!("   If the agent makes a mistake after receiving correct context,");
    println!("   the TRACE log proves it wasn't due to lack of information.");
    println!("");
    println!("   This is the foundation of AI agent governance:");
    println!("   \"If it wasn't emitted by the runtime, it didn't happen.\"");

    assert!(verify_a.is_valid);
    assert!(verify_b.is_valid);
    assert!(resolution_a.context_blocks.is_empty());
    assert!(!resolution_b.context_blocks.is_empty());
}

#[test]
fn detailed_context_matching_analysis() {
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║              CONTEXT MATCHING ANALYSIS: How CRA Decides                    ║");
    println!("╚════════════════════════════════════════════════════════════════════════════╝");

    let mut resolver = Resolver::new();
    let atlas = load_vib3_atlas();

    println!("\n📦 ATLAS CONTEXT BLOCKS:");
    println!("┌──────────────────────────┬──────────┬────────────┬─────────────────────────┐");
    println!("│ Context ID               │ Priority │ Mode       │ Keywords                │");
    println!("├──────────────────────────┼──────────┼────────────┼─────────────────────────┤");
    for block in &atlas.context_blocks {
        let mode = format!("{:?}", block.inject_mode);
        let keywords: Vec<_> = block.keywords.iter().take(3).cloned().collect();
        let kw_str = if keywords.is_empty() {
            "(none)".to_string()
        } else {
            keywords.join(", ")
        };
        println!("│ {:<24} │ {:>8} │ {:<10} │ {:<23} │",
            &block.context_id[..block.context_id.len().min(24)],
            block.priority,
            &mode[..mode.len().min(10)],
            &kw_str[..kw_str.len().min(23)]
        );
    }
    println!("└──────────────────────────┴──────────┴────────────┴─────────────────────────┘");

    resolver.load_atlas(atlas).unwrap();
    let session = resolver.create_session("analyzer", "context analysis").unwrap();

    let test_cases = vec![
        ("embed shader background", vec!["embed", "shader", "background"]),
        ("music visualizer audio reactive", vec!["music", "audio", "visualizer"]),
        ("customize UI controls", vec!["customize", "UI"]),
        ("clone repo modify defaults", vec!["clone", "repo", "modify"]),
        ("what geometry for torus", vec!["geometry", "torus"]),
        ("SDK npm integration", vec!["SDK", "npm"]),
    ];

    for (goal, expected_keywords) in test_cases {
        println!("\n┌─────────────────────────────────────────────────────────────────────────────┐");
        println!("│ GOAL: \"{}\"", goal);
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        println!("│ Keywords in goal: {:?}", expected_keywords);
        println!("│");

        let request = CARPRequest::new(
            session.clone(),
            "analyzer".to_string(),
            goal.to_string(),
        );

        let resolution = resolver.resolve(&request).unwrap();

        println!("│ MATCHED CONTEXT BLOCKS:");
        if resolution.context_blocks.is_empty() {
            println!("│   (none matched)");
        } else {
            for block in &resolution.context_blocks {
                println!("│   ✓ {} (priority {})", block.block_id, block.priority);
            }
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    // Final trace stats
    let trace = resolver.get_trace(&session).unwrap();
    let context_events: Vec<_> = trace.iter()
        .filter(|e| e.event_type == EventType::ContextInjected)
        .collect();

    println!("\n═══════════════════════════════════════════════════════════════════════════════");
    println!("📊 TRACE STATISTICS:");
    println!("   Total events: {}", trace.len());
    println!("   context.injected: {}", context_events.len());
    println!("   Unique contexts injected:");

    let mut seen = std::collections::HashSet::new();
    for event in &context_events {
        if let Some(ctx_id) = event.payload.get("context_id").and_then(|v| v.as_str()) {
            if seen.insert(ctx_id) {
                println!("     - {}", ctx_id);
            }
        }
    }

    let verification = resolver.verify_chain(&session).unwrap();
    println!("\n🔒 Chain Integrity: {}", if verification.is_valid { "✅ VALID" } else { "❌ INVALID" });

    assert!(verification.is_valid);
}
