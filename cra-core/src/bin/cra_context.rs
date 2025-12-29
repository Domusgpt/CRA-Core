//! CRA Context CLI - Query context for agent goals
//!
//! This CLI tool allows agents to query the CRA context registry
//! and receive rendered context for their goals.
//!
//! Usage:
//!     cra-context "I need to modify the hash computation"
//!     cra-context --atlas path/to/atlas.json "Add a new event type"
//!     cra-context --json "Working on trace module"

use clap::Parser;
use cra_core::{Resolver, CARPRequest, atlas::AtlasManifest};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "cra-context")]
#[command(about = "Query CRA context for agent goals")]
#[command(version)]
struct Args {
    /// The goal/task to get context for
    goal: String,

    /// Path to atlas JSON file (default: looks for cra-development.json)
    #[arg(short, long)]
    atlas: Option<PathBuf>,

    /// Output as JSON instead of rendered markdown
    #[arg(long)]
    json: bool,

    /// Show only context block IDs (no content)
    #[arg(long)]
    list_only: bool,

    /// Agent ID for session tracking
    #[arg(long, default_value = "cli-agent")]
    agent_id: String,

    /// Verbose output (show debug info)
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();

    // Find and load atlas
    let atlas = match load_atlas(&args.atlas, args.verbose) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error loading atlas: {}", e);
            std::process::exit(1);
        }
    };

    // Create resolver and load atlas
    let mut resolver = Resolver::new();
    if let Err(e) = resolver.load_atlas(atlas.clone()) {
        eprintln!("Error loading atlas into resolver: {}", e);
        std::process::exit(1);
    }

    // Create session
    let session_id = match resolver.create_session(&args.agent_id, "CLI query") {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error creating session: {}", e);
            std::process::exit(1);
        }
    };

    if args.verbose {
        eprintln!("Session: {}", &session_id[..8]);
        eprintln!("Atlas: {} v{}", atlas.name, atlas.version);
        eprintln!("Goal: \"{}\"", args.goal);
        eprintln!();
    }

    // Create request and resolve
    let request = CARPRequest::new(
        session_id,
        args.agent_id,
        args.goal,
    );

    let resolution = match resolver.resolve(&request) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error resolving request: {}", e);
            std::process::exit(1);
        }
    };

    // Output based on format
    if args.json {
        output_json(&resolution);
    } else if args.list_only {
        output_list(&resolution);
    } else {
        output_rendered(&resolution, args.verbose);
    }
}

fn load_atlas(path: &Option<PathBuf>, verbose: bool) -> Result<AtlasManifest, String> {
    // If path provided, use it
    if let Some(p) = path {
        if verbose {
            eprintln!("Loading atlas from: {}", p.display());
        }
        let content = std::fs::read_to_string(p)
            .map_err(|e| format!("Failed to read atlas file: {}", e))?;
        return serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse atlas JSON: {}", e));
    }

    // Try default locations
    let default_paths = [
        PathBuf::from("atlases/cra-development.json"),
        PathBuf::from("../atlases/cra-development.json"),
        PathBuf::from("../../atlases/cra-development.json"),
        PathBuf::from("cra-development.json"),
    ];

    for p in &default_paths {
        if p.exists() {
            if verbose {
                eprintln!("Found atlas at: {}", p.display());
            }
            let content = std::fs::read_to_string(p)
                .map_err(|e| format!("Failed to read atlas file: {}", e))?;
            return serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse atlas JSON: {}", e));
        }
    }

    Err("No atlas found. Specify with --atlas or place cra-development.json in atlases/".to_string())
}

fn output_json(resolution: &cra_core::CARPResolution) {
    #[derive(serde::Serialize)]
    struct JsonOutput {
        decision: String,
        context_blocks: Vec<JsonBlock>,
        rendered: String,
    }

    #[derive(serde::Serialize)]
    struct JsonBlock {
        id: String,
        name: String,
        priority: i32,
        source: String,
        content_type: String,
        content: String,
    }

    let output = JsonOutput {
        decision: format!("{:?}", resolution.decision),
        context_blocks: resolution.context_blocks.iter().map(|b| JsonBlock {
            id: b.block_id.clone(),
            name: b.name.clone(),
            priority: b.priority,
            source: b.source_atlas.clone(),
            content_type: b.content_type.clone(),
            content: b.content.clone(),
        }).collect(),
        rendered: resolution.render_context(),
    };

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

fn output_list(resolution: &cra_core::CARPResolution) {
    println!("Context blocks for goal:");
    println!();

    let mut blocks = resolution.context_blocks.clone();
    blocks.sort_by(|a, b| b.priority.cmp(&a.priority));

    for block in &blocks {
        println!("  {} (priority: {}, source: {})",
            block.block_id,
            block.priority,
            block.source_atlas
        );
    }

    println!();
    println!("Total: {} blocks", resolution.context_blocks.len());
}

fn output_rendered(resolution: &cra_core::CARPResolution, verbose: bool) {
    if verbose {
        eprintln!("Decision: {:?}", resolution.decision);
        eprintln!("Context blocks: {}", resolution.context_blocks.len());
        for block in &resolution.context_blocks {
            eprintln!("  - {} (priority: {})", block.block_id, block.priority);
        }
        eprintln!();
    }

    // Output the rendered context (this is what an LLM would receive)
    print!("{}", resolution.render_context());
}
