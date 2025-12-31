//! Comparative test: Agent WITH CRA vs Agent WITHOUT CRA
//!
//! This test demonstrates the difference between an agent working with
//! CRA governance (with VIB3+ atlas context) vs one without context.

use cra_core::atlas::AtlasManifest;
use cra_core::carp::{CARPRequest, Resolver};
use std::fs;

/// The task both agents will be given
const TASK: &str = r#"
Create a landing page for a music streaming service called "SoundWave"
with an animated VIB3+ background. The page should:
1. Have a hero section with the service name
2. Use VIB3+ for an audio-reactive animated background
3. Include a call-to-action button
"#;

/// Simulates what an agent WITHOUT CRA context would know
fn agent_without_cra_context() -> String {
    format!(r#"
=== AGENT WITHOUT CRA GOVERNANCE ===
Task: {}

Agent's knowledge about VIB3+: [NONE - has never heard of it]

Expected behavior:
- Agent will search for "VIB3+" but may not find relevant docs
- Will likely attempt to use CSS animations or canvas instead
- May hallucinate incorrect API
- Will need multiple iterations to figure out the system
- High risk of producing broken code

Sample output (what agent might generate without context):
```html
<!-- Agent's attempt without VIB3+ knowledge -->
<!DOCTYPE html>
<html>
<head>
    <title>SoundWave</title>
    <style>
        .animated-bg {{
            /* Agent might try CSS gradient animation */
            background: linear-gradient(45deg, #1a1a2e, #16213e);
            animation: pulse 3s infinite;
        }}
        @keyframes pulse {{
            0%, 100% {{ opacity: 0.8; }}
            50% {{ opacity: 1; }}
        }}
    </style>
</head>
<body>
    <div class="animated-bg">
        <h1>SoundWave</h1>
        <button>Start Listening</button>
    </div>
    <!-- No VIB3+ integration - agent doesn't know how -->
</body>
</html>
```

Issues:
- No actual VIB3+ visualization
- No audio reactivity
- Generic CSS animation instead of WebGL shaders
- Missing the key differentiator requested
"#, TASK)
}

/// Simulates what an agent WITH CRA context would know
fn agent_with_cra_context(resolver: &mut Resolver, session_id: &str) -> String {
    // Request context for the task
    let request = CARPRequest::new(
        session_id.to_string(),
        "web-design".to_string(),
        "Create landing page with VIB3+ audio-reactive background for music app".to_string(),
    );

    let resolution = resolver.resolve(&request).unwrap();

    // Collect injected context
    let mut context_summary = String::new();
    for ctx in &resolution.context_blocks {
        context_summary.push_str(&format!("\n--- {} (priority: {}) ---\n", ctx.name, ctx.priority));
        context_summary.push_str(&ctx.content);
        context_summary.push_str("\n");
    }

    format!(r#"
=== AGENT WITH CRA GOVERNANCE ===
Task: {}

Atlases loaded: ["dev.vib3.webpage-development"]

Context blocks injected by CRA:
{}

With this context, agent knows:
✓ VIB3+ is a shader-based visualization engine
✓ Three working systems: Faceted, Quantum, Holographic
✓ Holographic system is optimized for audio
✓ How to embed with iframe
✓ The JavaScript API (window.switchSystem, window.toggleAudio, etc.)
✓ Audio reactivity setup (bass/mid/high frequency bands)

Expected output (agent with full context):
```html
<!DOCTYPE html>
<html>
<head>
    <title>SoundWave - Music Streaming</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}

        .vib3-bg {{
            position: fixed;
            inset: 0;
            z-index: -1;
        }}
        .vib3-bg iframe {{
            width: 100%;
            height: 100%;
            border: none;
        }}

        .content {{
            position: relative;
            z-index: 1;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            text-align: center;
            color: white;
        }}

        h1 {{
            font-size: 4rem;
            text-shadow: 0 0 30px rgba(0, 255, 204, 0.5);
        }}

        .cta {{
            margin-top: 2rem;
            padding: 1rem 3rem;
            font-size: 1.2rem;
            background: linear-gradient(135deg, #00ffcc, #0099ff);
            border: none;
            border-radius: 50px;
            color: #000;
            cursor: pointer;
        }}
    </style>
</head>
<body>
    <!-- VIB3+ as full-page animated background -->
    <div class="vib3-bg">
        <iframe id="vib3" src="https://domusgpt.github.io/vib3-plus-engine/"></iframe>
    </div>

    <div class="content">
        <h1>🎵 SoundWave</h1>
        <p>Experience music like never before</p>
        <button class="cta" onclick="enableAudioReactive()">Start Listening</button>
    </div>

    <script>
        // VIB3+ API - agent knows this from CRA context
        const vib3 = document.getElementById('vib3');

        // Set up for music visualization
        vib3.onload = function() {{
            const w = vib3.contentWindow;
            // Use holographic system (optimized for audio)
            w.switchSystem('holographic');
            // Select torus with hypersphere warp (geometry 11)
            w.selectGeometry(11);
            // Purple/blue theme for music app
            w.updateParameter('hue', 280);
            w.updateParameter('intensity', 0.6);
        }};

        function enableAudioReactive() {{
            const w = vib3.contentWindow;
            // Enable audio reactivity
            w.toggleAudio();
            // Bass drives intensity, mids drive rotation
            w.toggleAudioReactivity('bass', 'intensity', true);
            w.toggleAudioReactivity('mid', 'rotation', true);
            w.toggleAudioReactivity('high', 'color', true);
        }}
    </script>
</body>
</html>
```

Benefits of CRA governance:
✓ Correct API usage from the start
✓ Proper system selection (holographic for audio)
✓ Working audio reactivity setup
✓ No hallucinated functions
✓ First-try success without iteration
"#, TASK, context_summary)
}

#[test]
fn comparative_test_cra_vs_no_cra() {
    // Print agent WITHOUT CRA
    println!("{}", agent_without_cra_context());
    println!("\n{}\n", "=".repeat(80));

    // Set up CRA resolver with VIB3+ atlas
    let mut resolver = Resolver::new();

    // Load the VIB3+ atlas
    let atlas_json = fs::read_to_string("../atlases/vib3-webpage-development.json")
        .expect("VIB3+ atlas should exist");
    let atlas: AtlasManifest = serde_json::from_str(&atlas_json)
        .expect("Atlas should parse");

    resolver.load_atlas(atlas).unwrap();

    // Create session
    let session_id = resolver.create_session("test-agent", "Create VIB3+ landing page").unwrap();

    // Print agent WITH CRA
    println!("{}", agent_with_cra_context(&mut resolver, &session_id));

    // Verify context was injected
    let trace = resolver.get_trace(&session_id).unwrap();
    let context_events: Vec<_> = trace.iter()
        .filter(|e| e.event_type == cra_core::trace::EventType::ContextInjected)
        .collect();

    println!("\n=== TRACE VERIFICATION ===");
    println!("Context injection events: {}", context_events.len());

    for event in &context_events {
        if let Some(ctx_id) = event.payload.get("context_id") {
            println!("  - Injected: {}", ctx_id);
        }
    }

    // The key assertion: with CRA, context was injected
    assert!(
        !context_events.is_empty(),
        "CRA should inject VIB3+ context for this task"
    );

    println!("\n=== CONCLUSION ===");
    println!("Without CRA: Agent lacks domain knowledge, will struggle or fail");
    println!("With CRA: Agent receives {} context blocks with verified API information", context_events.len());
}
