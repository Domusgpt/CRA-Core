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
// Serve Command
// =============================================================================

program
  .command('serve')
  .description('Start CRA HTTP server')
  .option('-p, --port <port>', 'Port to listen on', '3000')
  .option('-h, --host <host>', 'Host to bind to', '0.0.0.0')
  .option('-a, --atlases <paths>', 'Atlas paths (comma-separated)', './atlases')
  .option('-k, --api-key <key>', 'API key for authentication')
  .option('--no-cors', 'Disable CORS')
  .option('--no-websocket', 'Disable WebSocket trace streaming')
  .option('--rate-limit <limit>', 'Rate limit (requests per minute)', '100')
  .action(async (options) => {
    console.log(chalk.blue('Starting CRA server...'));

    try {
      // Dynamic import to avoid loading server package when not needed
      const { CRAServer } = await import('@cra/server');

      const server = new CRAServer({
        port: parseInt(options.port),
        host: options.host,
        atlasPaths: options.atlases.split(','),
        apiKey: options.apiKey,
        cors: options.cors,
        enableWebSocket: options.websocket,
        rateLimit: parseInt(options.rateLimit),
      });

      await server.start();

      console.log(chalk.green(`\nCRA server running on http://${options.host}:${options.port}`));
      console.log(chalk.gray('\nEndpoints:'));
      console.log(chalk.gray('  POST /v1/resolve   - CARP resolution'));
      console.log(chalk.gray('  POST /v1/execute   - Action execution'));
      console.log(chalk.gray('  POST /v1/adapt/:p  - Platform adaptation'));
      console.log(chalk.gray('  GET  /health       - Health check'));
      if (options.websocket) {
        console.log(chalk.gray(`  WS   /v1/trace     - Trace streaming`));
      }
      console.log(chalk.gray('\nPress Ctrl+C to stop'));

      // Handle shutdown
      process.on('SIGINT', async () => {
        console.log(chalk.yellow('\nShutting down...'));
        await server.stop();
        process.exit(0);
      });

    } catch (error) {
      console.error(chalk.red(`Failed to start server: ${error}`));
      process.exit(1);
    }
  });

// =============================================================================
// Doctor Command
// =============================================================================

program
  .command('doctor')
  .description('Diagnose CRA installation and configuration')
  .action(async () => {
    console.log(chalk.blue('CRA Doctor - Checking installation...\n'));

    const checks: { name: string; status: 'ok' | 'warn' | 'error'; message: string }[] = [];

    // Check Node.js version
    const nodeVersion = process.version;
    const nodeMajor = parseInt(nodeVersion.slice(1));
    if (nodeMajor >= 20) {
      checks.push({ name: 'Node.js version', status: 'ok', message: `${nodeVersion} (>= 20.0.0)` });
    } else {
      checks.push({ name: 'Node.js version', status: 'error', message: `${nodeVersion} (requires >= 20.0.0)` });
    }

    // Check config directory
    if (fs.existsSync('config/cra.json')) {
      checks.push({ name: 'Configuration', status: 'ok', message: 'config/cra.json found' });
    } else {
      checks.push({ name: 'Configuration', status: 'warn', message: 'config/cra.json not found (run cra init)' });
    }

    // Check atlases directory
    if (fs.existsSync('atlases')) {
      const atlases = fs.readdirSync('atlases', { withFileTypes: true })
        .filter(d => d.isDirectory()).length;
      if (atlases > 0) {
        checks.push({ name: 'Atlases', status: 'ok', message: `${atlases} atlas(es) found` });
      } else {
        checks.push({ name: 'Atlases', status: 'warn', message: 'No atlases found in ./atlases/' });
      }
    } else {
      checks.push({ name: 'Atlases', status: 'warn', message: './atlases/ directory not found' });
    }

    // Check traces directory
    if (fs.existsSync('traces')) {
      const traces = fs.readdirSync('traces').filter(f => f.endsWith('.trace.jsonl')).length;
      checks.push({ name: 'Traces', status: 'ok', message: `${traces} trace file(s)` });
    } else {
      checks.push({ name: 'Traces', status: 'warn', message: './traces/ directory not found' });
    }

    // Check agents.md
    if (fs.existsSync('agents.md')) {
      checks.push({ name: 'Agents contract', status: 'ok', message: 'agents.md found' });
    } else {
      checks.push({ name: 'Agents contract', status: 'warn', message: 'agents.md not found' });
    }

    // Print results
    for (const check of checks) {
      const icon = check.status === 'ok' ? chalk.green('✓') :
                   check.status === 'warn' ? chalk.yellow('!') : chalk.red('✗');
      console.log(`${icon} ${check.name}: ${chalk.gray(check.message)}`);
    }

    const errors = checks.filter(c => c.status === 'error').length;
    const warns = checks.filter(c => c.status === 'warn').length;

    console.log();
    if (errors > 0) {
      console.log(chalk.red(`${errors} error(s) found. Please fix before proceeding.`));
      process.exit(1);
    } else if (warns > 0) {
      console.log(chalk.yellow(`${warns} warning(s). Run 'cra init' to set up missing components.`));
    } else {
      console.log(chalk.green('All checks passed!'));
    }
  });

// =============================================================================
// Atlas Create Command
// =============================================================================

atlasCmd
  .command('create <name>')
  .description('Create a new atlas from template')
  .option('-d, --domain <domain>', 'Primary domain for the atlas')
  .option('-t, --template <template>', 'Template type (basic, api, ops)', 'basic')
  .option('-o, --output <path>', 'Output directory', './atlases')
  .option('--author <author>', 'Author name', 'Your Organization')
  .option('--description <desc>', 'Atlas description')
  .option('-y, --yes', 'Skip confirmation prompts')
  .action(async (name, options) => {
    const atlasPath = path.join(options.output, name);
    const domain = options.domain ?? name.replace(/-/g, '.').toLowerCase();
    const atlasId = `atlas.${name.replace(/-/g, '_').toLowerCase()}`;

    if (fs.existsSync(atlasPath)) {
      console.log(chalk.red(`Atlas already exists at ${atlasPath}`));
      process.exit(1);
    }

    console.log(chalk.blue(`\nCreating atlas: ${chalk.bold(name)}`));
    console.log(chalk.gray(`  Template: ${options.template}`));
    console.log(chalk.gray(`  Domain: ${domain}`));
    console.log(chalk.gray(`  Output: ${atlasPath}`));
    console.log();

    // Create directory structure
    const dirs = [
      atlasPath,
      path.join(atlasPath, 'context'),
      path.join(atlasPath, 'adapters'),
      path.join(atlasPath, 'tests'),
    ];

    for (const dir of dirs) {
      fs.mkdirSync(dir, { recursive: true });
    }

    // Generate atlas.json based on template
    const manifest = generateAtlasManifest(name, atlasId, domain, options);
    fs.writeFileSync(
      path.join(atlasPath, 'atlas.json'),
      JSON.stringify(manifest, null, 2)
    );
    console.log(chalk.green(`  ✓ Created atlas.json`));

    // Generate context files based on template
    const contextFiles = generateContextFiles(name, domain, options.template);
    for (const [filename, content] of Object.entries(contextFiles)) {
      fs.writeFileSync(path.join(atlasPath, 'context', filename), content);
      console.log(chalk.green(`  ✓ Created context/${filename}`));
    }

    // Generate adapter configs
    const adapterFiles = generateAdapterFiles(name, domain);
    for (const [filename, content] of Object.entries(adapterFiles)) {
      fs.writeFileSync(path.join(atlasPath, 'adapters', filename), content);
      console.log(chalk.green(`  ✓ Created adapters/${filename}`));
    }

    // Generate test file
    const testContent = generateTestFile(name, atlasId);
    fs.writeFileSync(path.join(atlasPath, 'tests', 'atlas.test.ts'), testContent);
    console.log(chalk.green(`  ✓ Created tests/atlas.test.ts`));

    // Print summary
    console.log(chalk.blue(`\n✓ Atlas "${name}" created successfully!\n`));
    console.log(chalk.gray('Next steps:'));
    console.log(chalk.gray(`  1. Edit ${atlasPath}/atlas.json to customize metadata`));
    console.log(chalk.gray(`  2. Add context in ${atlasPath}/context/`));
    console.log(chalk.gray(`  3. Define actions and policies in atlas.json`));
    console.log(chalk.gray(`  4. Run: cra atlas validate ${atlasPath}`));
    console.log();
  });

// Atlas template generators
function generateAtlasManifest(
  name: string,
  atlasId: string,
  domain: string,
  options: { template: string; author: string; description?: string }
): object {
  const description = options.description ?? `${name} operations atlas`;

  const baseManifest = {
    atlas_version: '0.1',
    metadata: {
      id: atlasId,
      name: name,
      version: '0.1.0',
      description,
      authors: [{ name: options.author }],
      license: { type: 'free', spdx: 'MIT' },
      keywords: [domain, 'cra', 'atlas'],
    },
    domains: [
      {
        id: domain,
        name: name.replace(/-/g, ' '),
        description: `${name} domain`,
        risk_tier: 'medium' as const,
      },
    ],
    context_packs: [
      {
        id: `${domain}.overview`,
        domain,
        source: 'context/overview.md',
        ttl_seconds: 3600,
        tags: ['overview', 'getting-started'],
      },
    ],
    policies: [
      {
        id: 'default-policy',
        name: 'Default Policy',
        version: '1.0.0',
        rules: [
          {
            id: 'allow-low-risk',
            description: 'Allow low-risk operations',
            condition: { type: 'risk_tier', operator: 'in', value: ['low', 'medium'] },
            effect: 'allow',
            priority: 100,
          },
          {
            id: 'require-approval-high-risk',
            description: 'Require approval for high-risk operations',
            condition: { type: 'risk_tier', operator: 'in', value: ['high', 'critical'] },
            effect: 'require_approval',
            priority: 50,
          },
        ],
      },
    ],
    actions: [] as object[],
    adapters: [
      { platform: 'openai', config_file: 'adapters/openai.json' },
      { platform: 'claude', config_file: 'adapters/claude.json' },
    ],
    tests: [
      { id: 'conformance', name: 'Conformance Tests', type: 'conformance', test_files: ['tests/atlas.test.ts'] },
    ],
  };

  // Add template-specific actions
  if (options.template === 'api') {
    baseManifest.actions = [
      {
        id: `${domain}.get`,
        type: 'api.get',
        name: 'GET Request',
        description: `Make a GET request to ${name} API`,
        domain,
        risk_tier: 'low',
        schema: {
          type: 'object',
          properties: {
            endpoint: { type: 'string', description: 'API endpoint path' },
            params: { type: 'object', description: 'Query parameters' },
          },
          required: ['endpoint'],
        },
      },
      {
        id: `${domain}.post`,
        type: 'api.post',
        name: 'POST Request',
        description: `Make a POST request to ${name} API`,
        domain,
        risk_tier: 'medium',
        schema: {
          type: 'object',
          properties: {
            endpoint: { type: 'string', description: 'API endpoint path' },
            body: { type: 'object', description: 'Request body' },
          },
          required: ['endpoint', 'body'],
        },
      },
      {
        id: `${domain}.delete`,
        type: 'api.delete',
        name: 'DELETE Request',
        description: `Make a DELETE request to ${name} API`,
        domain,
        risk_tier: 'high',
        schema: {
          type: 'object',
          properties: {
            endpoint: { type: 'string', description: 'API endpoint path' },
            id: { type: 'string', description: 'Resource ID to delete' },
          },
          required: ['endpoint', 'id'],
        },
      },
    ];
  } else if (options.template === 'ops') {
    baseManifest.actions = [
      {
        id: `${domain}.list`,
        type: 'ops.list',
        name: 'List Resources',
        description: `List ${name} resources`,
        domain,
        risk_tier: 'low',
        schema: {
          type: 'object',
          properties: {
            filter: { type: 'string', description: 'Filter expression' },
            limit: { type: 'number', description: 'Maximum results' },
          },
        },
      },
      {
        id: `${domain}.create`,
        type: 'ops.create',
        name: 'Create Resource',
        description: `Create a new ${name} resource`,
        domain,
        risk_tier: 'medium',
        schema: {
          type: 'object',
          properties: {
            name: { type: 'string', description: 'Resource name' },
            config: { type: 'object', description: 'Resource configuration' },
          },
          required: ['name'],
        },
      },
      {
        id: `${domain}.update`,
        type: 'ops.update',
        name: 'Update Resource',
        description: `Update an existing ${name} resource`,
        domain,
        risk_tier: 'medium',
        schema: {
          type: 'object',
          properties: {
            id: { type: 'string', description: 'Resource ID' },
            updates: { type: 'object', description: 'Fields to update' },
          },
          required: ['id', 'updates'],
        },
      },
      {
        id: `${domain}.delete`,
        type: 'ops.delete',
        name: 'Delete Resource',
        description: `Delete a ${name} resource`,
        domain,
        risk_tier: 'high',
        schema: {
          type: 'object',
          properties: {
            id: { type: 'string', description: 'Resource ID to delete' },
            force: { type: 'boolean', description: 'Force deletion' },
          },
          required: ['id'],
        },
      },
    ];
  }

  return baseManifest;
}

function generateContextFiles(name: string, domain: string, template: string): Record<string, string> {
  const files: Record<string, string> = {};

  // Overview context
  files['overview.md'] = `# ${name} Overview

This context pack provides guidelines for working with ${name}.

## Domain: ${domain}

### Capabilities

- List resources and their current state
- Create new resources with proper configuration
- Update existing resources safely
- Delete resources with appropriate safeguards

### Best Practices

1. **Always verify before destructive operations**
   - Check resource dependencies before deletion
   - Use \`force\` flag only when necessary

2. **Use appropriate risk tiers**
   - Read operations: low risk
   - Create/update operations: medium risk
   - Delete operations: high risk

3. **Include proper context**
   - Provide clear descriptions for operations
   - Include relevant identifiers and metadata

## Common Patterns

### Listing Resources

\`\`\`
Goal: "List all ${domain} resources"
Risk tier: low
Actions: ${domain}.list
\`\`\`

### Creating Resources

\`\`\`
Goal: "Create a new ${domain} resource"
Risk tier: medium
Actions: ${domain}.create
\`\`\`

## Error Handling

- Check for existence before operations
- Handle rate limiting gracefully
- Provide clear error messages

## Security Considerations

- Validate all inputs
- Use least-privilege access
- Audit high-risk operations
`;

  // Add template-specific context files
  if (template === 'api') {
    files['api-reference.md'] = `# ${name} API Reference

## Authentication

All API requests require authentication via API key or OAuth token.

## Endpoints

### GET Requests

Low-risk operations for reading data.

### POST Requests

Medium-risk operations for creating resources.

### DELETE Requests

High-risk operations requiring approval for production resources.

## Rate Limits

- Standard: 100 requests/minute
- Burst: 200 requests/minute

## Error Codes

| Code | Description |
|------|-------------|
| 400 | Bad Request |
| 401 | Unauthorized |
| 403 | Forbidden |
| 404 | Not Found |
| 429 | Rate Limited |
| 500 | Internal Error |
`;
  } else if (template === 'ops') {
    files['operations-guide.md'] = `# ${name} Operations Guide

## Resource Lifecycle

1. **Create**: Initialize new resources
2. **Update**: Modify existing resources
3. **Delete**: Remove resources safely

## Operational Patterns

### Batch Operations

For bulk operations, process in batches of 10-50 items.

### Rollback Procedures

1. Document current state before changes
2. Keep deletion audit logs
3. Implement undo where possible

## Monitoring

- Track resource creation/deletion rates
- Alert on unusual patterns
- Log all high-risk operations

## Compliance

- Maintain audit trail
- Enforce naming conventions
- Apply proper tags/labels
`;
  }

  return files;
}

function generateAdapterFiles(name: string, domain: string): Record<string, string> {
  const files: Record<string, string> = {};

  files['openai.json'] = JSON.stringify({
    platform: 'openai',
    tool_prefix: domain.replace(/\./g, '_'),
    tool_mappings: [
      {
        action_id: `${domain}.list`,
        tool_name: `${domain.replace(/\./g, '_')}_list`,
      },
    ],
    system_prompt_additions: [
      `You have access to ${name} operations.`,
      `Use the ${domain} tools to interact with resources.`,
    ],
  }, null, 2);

  files['claude.json'] = JSON.stringify({
    platform: 'claude',
    tool_prefix: domain.replace(/\./g, '_'),
    tool_mappings: [
      {
        action_id: `${domain}.list`,
        tool_name: `${domain.replace(/\./g, '_')}_list`,
      },
    ],
    system_prompt_additions: [
      `You have access to ${name} operations.`,
      `Use the ${domain} tools to interact with resources.`,
    ],
  }, null, 2);

  return files;
}

function generateTestFile(name: string, atlasId: string): string {
  return `/**
 * ${name} Atlas Conformance Tests
 */

import { describe, it, expect } from 'vitest';
import { AtlasLoader } from '@cra/atlas';
import * as path from 'path';

describe('${name} Atlas', () => {
  const atlasPath = path.join(__dirname, '..');
  let loader: AtlasLoader;

  beforeAll(() => {
    loader = new AtlasLoader({ validate: true });
  });

  it('should load successfully', async () => {
    const atlas = await loader.load(atlasPath);
    expect(atlas.manifest.metadata.id).toBe('${atlasId}');
  });

  it('should have valid domains', async () => {
    const atlas = await loader.load(atlasPath);
    expect(atlas.manifest.domains.length).toBeGreaterThan(0);
  });

  it('should have valid context packs', async () => {
    const atlas = await loader.load(atlasPath);
    expect(atlas.manifest.context_packs.length).toBeGreaterThan(0);
  });

  it('should have valid policies', async () => {
    const atlas = await loader.load(atlasPath);
    expect(atlas.manifest.policies.length).toBeGreaterThan(0);
  });

  it('should validate against schema', async () => {
    const result = await loader.validate(atlasPath);
    expect(result.valid).toBe(true);
  });
});
`;
}

// =============================================================================
// Run CLI
// =============================================================================

program.parse();
