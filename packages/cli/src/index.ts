#!/usr/bin/env node

/**
 * CRA CLI
 *
 * Telemetry-first command line interface for Context Registry Agents.
 * Asynchronous operation with continuous structured telemetry output (JSONL).
 */

import { Command } from 'commander';
import chalk from 'chalk';
import * as fs from 'fs';
import * as path from 'path';
import { CRARuntime, createRequest } from '@cra/runtime';
import { loadTraceFile, replayTrace, diffTraces } from '@cra/trace';
import { AtlasLoader } from '@cra/atlas';
import type { TRACEEvent } from '@cra/protocol';

// =============================================================================
// CLI Setup
// =============================================================================

const program = new Command();

program
  .name('cra')
  .description('CRA CLI - Context Registry Agents command line interface')
  .version('0.1.0');

// =============================================================================
// Init Command
// =============================================================================

program
  .command('init')
  .description('Initialize a new CRA project')
  .option('-f, --force', 'Overwrite existing files')
  .action(async (options) => {
    console.log(chalk.blue('Initializing CRA project...'));

    const dirs = ['config', 'traces', 'atlases'];
    const files = [
      { path: 'agents.md', content: AGENTS_MD_TEMPLATE },
      { path: 'config/cra.json', content: CONFIG_TEMPLATE },
    ];

    // Create directories
    for (const dir of dirs) {
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
        console.log(chalk.green(`  Created ${dir}/`));
      } else {
        console.log(chalk.yellow(`  ${dir}/ already exists`));
      }
    }

    // Create files
    for (const file of files) {
      if (!fs.existsSync(file.path) || options.force) {
        fs.writeFileSync(file.path, file.content);
        console.log(chalk.green(`  Created ${file.path}`));
      } else {
        console.log(chalk.yellow(`  ${file.path} already exists (use --force to overwrite)`));
      }
    }

    console.log(chalk.blue('\nCRA project initialized successfully!'));
    console.log(chalk.gray('\nNext steps:'));
    console.log(chalk.gray('  1. Add atlases to ./atlases/'));
    console.log(chalk.gray('  2. Run `cra resolve "your goal"` to test'));
    console.log(chalk.gray('  3. Run `cra trace tail` to watch telemetry'));
  });

// =============================================================================
// Resolve Command
// =============================================================================

program
  .command('resolve <goal>')
  .description('Resolve context and permissions for a goal')
  .option('-r, --risk <tier>', 'Risk tier (low, medium, high, critical)', 'medium')
  .option('-d, --domains <domains>', 'Context domains (comma-separated)')
  .option('-a, --atlases <paths>', 'Atlas paths (comma-separated)')
  .option('-j, --json', 'Output as JSON')
  .option('--trace', 'Show trace events')
  .action(async (goal, options) => {
    const runtime = new CRARuntime({
      trace_to_file: true,
      trace_dir: './traces',
    });

    // Load atlases
    const atlasPaths = options.atlases?.split(',') ?? ['./atlases'];
    for (const atlasPath of atlasPaths) {
      if (fs.existsSync(atlasPath)) {
        try {
          await runtime.loadAtlas(atlasPath);
          console.log(chalk.gray(`Loaded atlas from ${atlasPath}`));
        } catch (error) {
          console.error(chalk.red(`Failed to load atlas from ${atlasPath}: ${error}`));
        }
      }
    }

    // Subscribe to trace events if requested
    if (options.trace) {
      runtime.getTrace().on('event', (event: TRACEEvent) => {
        console.log(chalk.gray(JSON.stringify(event)));
      });
    }

    // Create request
    const request = createRequest(
      'resolve',
      {
        agent_id: 'cli',
        session_id: runtime.getSessionId(),
      },
      {
        task: {
          goal,
          risk_tier: options.risk,
          context_hints: options.domains?.split(','),
        },
      }
    );

    // Resolve
    const result = await runtime.resolve(request);

    // Output
    if (options.json) {
      console.log(JSON.stringify(result, null, 2));
    } else {
      if ('error' in result) {
        console.log(chalk.red(`\nError: ${result.error.code}`));
        console.log(chalk.red(result.error.message));
      } else {
        console.log(chalk.green(`\nResolution: ${result.resolution_id}`));
        console.log(chalk.blue(`Decision: ${result.decision.type}`));
        console.log(chalk.gray(`Context blocks: ${result.context_blocks.length}`));
        console.log(chalk.gray(`Allowed actions: ${result.allowed_actions.length}`));
        console.log(chalk.gray(`Policies applied: ${result.policies_applied.length}`));

        if (result.context_blocks.length > 0) {
          console.log(chalk.blue('\nContext:'));
          for (const block of result.context_blocks) {
            console.log(chalk.gray(`  - [${block.domain}] ${block.content.slice(0, 100)}...`));
          }
        }

        if (result.allowed_actions.length > 0) {
          console.log(chalk.blue('\nAllowed Actions:'));
          for (const action of result.allowed_actions.slice(0, 10)) {
            console.log(chalk.gray(`  - ${action.action_type}: ${action.description}`));
          }
          if (result.allowed_actions.length > 10) {
            console.log(chalk.gray(`  ... and ${result.allowed_actions.length - 10} more`));
          }
        }

        console.log(chalk.gray(`\nTrace: ${result.telemetry_link.trace_id}`));
      }
    }

    await runtime.shutdown();
  });

// =============================================================================
// Execute Command
// =============================================================================

program
  .command('execute <action-id>')
  .description('Execute a permitted action')
  .option('-r, --resolution <id>', 'Resolution ID (required)')
  .option('-p, --params <json>', 'Action parameters as JSON')
  .option('-j, --json', 'Output as JSON')
  .action(async (_actionId, options) => {
    if (!options.resolution) {
      console.error(chalk.red('Error: --resolution is required'));
      process.exit(1);
    }

    const runtime = new CRARuntime({
      trace_to_file: true,
      trace_dir: './traces',
    });

    // Note: In a real implementation, we would load the resolution from storage
    console.log(chalk.yellow('Note: Execute requires a valid resolution from a prior resolve'));

    await runtime.shutdown();
  });

// =============================================================================
// Trace Commands
// =============================================================================

const traceCmd = program
  .command('trace')
  .description('Trace telemetry commands');

traceCmd
  .command('tail')
  .description('Tail the trace event stream')
  .option('-f, --file <path>', 'Watch specific trace file')
  .option('-n, --lines <count>', 'Show last N events', '10')
  .option('--filter <types>', 'Filter by event types (comma-separated)')
  .action(async (options) => {
    console.log(chalk.blue('Tailing trace events...\n'));

    if (options.file) {
      // Watch specific file
      const events = await loadTraceFile(options.file);
      const lastN = parseInt(options.lines);
      const filtered = options.filter
        ? events.filter(e => options.filter.split(',').some((t: string) => e.event_type.includes(t)))
        : events;

      for (const event of filtered.slice(-lastN)) {
        printEvent(event);
      }

      // Watch for new events
      console.log(chalk.gray('\nWatching for new events... (Ctrl+C to stop)\n'));
      fs.watchFile(options.file, async () => {
        const newEvents = await loadTraceFile(options.file);
        for (const event of newEvents.slice(filtered.length)) {
          printEvent(event);
        }
      });
    } else {
      // List recent trace files
      const traceDir = './traces';
      if (!fs.existsSync(traceDir)) {
        console.log(chalk.yellow('No traces directory found. Run `cra init` first.'));
        return;
      }

      const files = fs.readdirSync(traceDir)
        .filter(f => f.endsWith('.trace.jsonl'))
        .sort()
        .reverse();

      if (files.length === 0) {
        console.log(chalk.yellow('No trace files found. Run `cra resolve` to generate traces.'));
        return;
      }

      console.log(chalk.blue('Recent trace files:'));
      for (const file of files.slice(0, 5)) {
        console.log(chalk.gray(`  ${file}`));
      }

      // Show events from most recent file
      const latestFile = path.join(traceDir, files[0]);
      console.log(chalk.blue(`\nEvents from ${files[0]}:\n`));

      const events = await loadTraceFile(latestFile);
      for (const event of events.slice(-parseInt(options.lines))) {
        printEvent(event);
      }
    }
  });

traceCmd
  .command('replay <file>')
  .description('Replay a trace file')
  .option('-s, --speed <multiplier>', 'Playback speed', '1.0')
  .option('--start <event-id>', 'Start from event ID')
  .option('--stop <event-id>', 'Stop at event ID')
  .action(async (file, options) => {
    console.log(chalk.blue(`Replaying ${file}...\n`));

    const speed = parseFloat(options.speed);

    for await (const { event, position, total, delay_ms } of replayTrace(file, {
      speed,
      start_at: options.start,
      stop_at: options.stop,
    })) {
      printEvent(event);
      process.stdout.write(chalk.gray(` [${position}/${total}]`));
      if (delay_ms > 0) {
        process.stdout.write(chalk.gray(` +${delay_ms.toFixed(0)}ms`));
      }
      console.log();
    }

    console.log(chalk.blue('\nReplay complete.'));
  });

traceCmd
  .command('diff <file1> <file2>')
  .description('Compare two trace files')
  .option('--ignore <fields>', 'Fields to ignore (comma-separated)')
  .action(async (file1, file2, options) => {
    console.log(chalk.blue(`Comparing traces...\n`));

    const events1 = await loadTraceFile(file1);
    const events2 = await loadTraceFile(file2);

    const diff = diffTraces(events1, events2, {
      ignore_fields: options.ignore?.split(','),
    });

    console.log(chalk.blue('Summary:'));
    console.log(chalk.gray(`  Events added: ${diff.summary.events_added}`));
    console.log(chalk.gray(`  Events removed: ${diff.summary.events_removed}`));
    console.log(chalk.gray(`  Events modified: ${diff.summary.events_modified}`));
    console.log(chalk.gray(`  Compatibility: ${diff.compatibility}`));

    if (diff.differences.length > 0) {
      console.log(chalk.blue('\nDifferences:'));
      for (const d of diff.differences.slice(0, 20)) {
        const color = d.severity === 'error' ? chalk.red : d.severity === 'warning' ? chalk.yellow : chalk.gray;
        console.log(color(`  [${d.type}] ${d.path}: ${d.message}`));
      }
      if (diff.differences.length > 20) {
        console.log(chalk.gray(`  ... and ${diff.differences.length - 20} more`));
      }
    }
  });

traceCmd
  .command('verify <file>')
  .description('Verify trace integrity')
  .action(async (file) => {
    console.log(chalk.blue(`Verifying ${file}...\n`));

    const events = await loadTraceFile(file);
    const { valid, errors } = await import('@cra/trace').then(m => m.verifyChain(events));

    if (valid) {
      console.log(chalk.green('✓ Trace integrity verified'));
      console.log(chalk.gray(`  Events: ${events.length}`));
      console.log(chalk.gray(`  Chain: intact`));
    } else {
      console.log(chalk.red('✗ Trace integrity check failed'));
      for (const error of errors) {
        console.log(chalk.red(`  ${error}`));
      }
    }
  });

// =============================================================================
// Atlas Commands
// =============================================================================

const atlasCmd = program
  .command('atlas')
  .description('Atlas management commands');

atlasCmd
  .command('validate <path>')
  .description('Validate an atlas')
  .action(async (atlasPath) => {
    console.log(chalk.blue(`Validating ${atlasPath}...\n`));

    const loader = new AtlasLoader({ validate: true });

    try {
      const atlas = await loader.load(atlasPath);
      console.log(chalk.green('✓ Atlas is valid'));
      console.log(chalk.gray(`  ID: ${atlas.manifest.metadata.id}`));
      console.log(chalk.gray(`  Version: ${atlas.manifest.metadata.version}`));
      console.log(chalk.gray(`  Domains: ${atlas.manifest.domains.length}`));
      console.log(chalk.gray(`  Context packs: ${atlas.manifest.context_packs.length}`));
      console.log(chalk.gray(`  Actions: ${atlas.manifest.actions.length}`));
      console.log(chalk.gray(`  Policies: ${atlas.manifest.policies.length}`));
    } catch (error) {
      console.log(chalk.red(`✗ Validation failed: ${error}`));
      process.exit(1);
    }
  });

atlasCmd
  .command('list')
  .description('List installed atlases')
  .option('-p, --path <path>', 'Atlas directory', './atlases')
  .action(async (options) => {
    const atlasDir = options.path;

    if (!fs.existsSync(atlasDir)) {
      console.log(chalk.yellow(`No atlases directory at ${atlasDir}`));
      return;
    }

    const entries = fs.readdirSync(atlasDir, { withFileTypes: true });
    const loader = new AtlasLoader({ validate: false });

    console.log(chalk.blue('Installed Atlases:\n'));

    for (const entry of entries) {
      if (entry.isDirectory()) {
        const atlasPath = path.join(atlasDir, entry.name);
        try {
          const atlas = await loader.load(atlasPath);
          console.log(chalk.green(`  ${atlas.manifest.metadata.id}@${atlas.manifest.metadata.version}`));
          console.log(chalk.gray(`    ${atlas.manifest.metadata.description}`));
        } catch {
          console.log(chalk.yellow(`  ${entry.name} (invalid)`));
        }
      }
    }
  });

// =============================================================================
// Config Command
// =============================================================================

program
  .command('config')
  .description('Show current configuration')
  .action(async () => {
    const configPath = 'config/cra.json';

    if (!fs.existsSync(configPath)) {
      console.log(chalk.yellow('No configuration found. Run `cra init` first.'));
      return;
    }

    const config = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
    console.log(chalk.blue('Current Configuration:\n'));
    console.log(JSON.stringify(config, null, 2));
  });

// =============================================================================
// Stats Command
// =============================================================================

program
  .command('stats')
  .description('Show runtime statistics')
  .action(async () => {
    const traceDir = './traces';

    if (!fs.existsSync(traceDir)) {
      console.log(chalk.yellow('No traces directory found.'));
      return;
    }

    const files = fs.readdirSync(traceDir).filter(f => f.endsWith('.trace.jsonl'));

    let totalEvents = 0;
    let totalResolutions = 0;
    let totalActions = 0;

    for (const file of files) {
      const events = await loadTraceFile(path.join(traceDir, file));
      totalEvents += events.length;
      totalResolutions += events.filter(e => e.event_type === 'carp.resolution.completed').length;
      totalActions += events.filter(e => e.event_type === 'carp.action.completed').length;
    }

    console.log(chalk.blue('CRA Statistics:\n'));
    console.log(chalk.gray(`  Trace files: ${files.length}`));
    console.log(chalk.gray(`  Total events: ${totalEvents}`));
    console.log(chalk.gray(`  Resolutions: ${totalResolutions}`));
    console.log(chalk.gray(`  Actions: ${totalActions}`));
  });

// =============================================================================
// Helper Functions
// =============================================================================

function printEvent(event: TRACEEvent): void {
  const time = new Date(event.timestamp).toISOString().slice(11, 23);
  const severity = event.severity === 'error' ? chalk.red : event.severity === 'warn' ? chalk.yellow : chalk.gray;
  const type = chalk.blue(event.event_type.padEnd(35));

  process.stdout.write(`${chalk.gray(time)} ${severity(event.severity.padEnd(5))} ${type}`);

  // Print key payload fields
  const payloadStr = JSON.stringify(event.payload);
  if (payloadStr.length > 60) {
    process.stdout.write(chalk.gray(payloadStr.slice(0, 60) + '...'));
  } else {
    process.stdout.write(chalk.gray(payloadStr));
  }

  console.log();
}

// =============================================================================
// Templates
// =============================================================================

const AGENTS_MD_TEMPLATE = `# agents.md — CRA Contract

This file defines the operational rules for AI agents in this project.

## Rules

1. **Always resolve via CARP** — Before taking any action, request context and permissions from the CRA runtime.

2. **Never guess tool usage** — Only use tools and actions that have been explicitly permitted in a CARP resolution.

3. **TRACE is authoritative** — The telemetry stream is the source of truth. LLM narration is advisory, not authoritative.

## Quick Reference

\`\`\`bash
# Resolve context for a goal
cra resolve "your goal here"

# Watch trace events
cra trace tail

# Verify trace integrity
cra trace verify ./traces/your-trace.jsonl
\`\`\`

## Configuration

See \`config/cra.json\` for runtime settings.
`;

const CONFIG_TEMPLATE = JSON.stringify({
  version: '0.1',
  runtime: {
    trace_dir: './traces',
    trace_to_file: true,
    default_ttl_seconds: 300,
    max_context_tokens: 8192,
    max_actions_per_resolution: 50,
  },
  atlases: {
    paths: ['./atlases'],
    auto_load: true,
  },
  telemetry: {
    format: 'jsonl',
    console_output: true,
  },
}, null, 2);

// =============================================================================
// Run CLI
// =============================================================================

program.parse();
