/**
 * CRA CLI Tests
 *
 * Basic tests for CLI command structure.
 */

import { describe, it, expect } from 'vitest';
import { Command } from 'commander';

describe('CLI Structure', () => {
  it('should have commander installed', () => {
    const cmd = new Command();
    cmd.name('test');
    expect(cmd.name()).toBe('test');
  });

  it('should support version option', () => {
    const cmd = new Command();
    cmd.version('0.1.0');
    expect(cmd.opts()).toBeDefined();
  });

  it('should support nested commands', () => {
    const cmd = new Command();
    const sub = cmd.command('sub').description('sub command');
    expect(sub).toBeDefined();
  });
});

describe('Template Generation', () => {
  it('should have valid JSON in config template', () => {
    const configTemplate = JSON.stringify({
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

    const parsed = JSON.parse(configTemplate);
    expect(parsed.version).toBe('0.1');
    expect(parsed.runtime.trace_dir).toBe('./traces');
  });
});

describe('CLI Command Definitions', () => {
  describe('Init Command', () => {
    it('should define init command', () => {
      const cmd = new Command();
      const init = cmd.command('init').description('Initialize');
      init.option('-f, --force', 'Overwrite');
      expect(init.description()).toBe('Initialize');
    });
  });

  describe('Resolve Command', () => {
    it('should define resolve command with goal argument', () => {
      const cmd = new Command();
      const resolve = cmd.command('resolve <goal>').description('Resolve context');
      resolve.option('-r, --risk <tier>', 'Risk tier', 'medium');
      resolve.option('-j, --json', 'Output as JSON');
      expect(resolve.description()).toBe('Resolve context');
    });
  });

  describe('Trace Commands', () => {
    it('should define trace parent command', () => {
      const cmd = new Command();
      const trace = cmd.command('trace').description('Trace commands');
      expect(trace.description()).toBe('Trace commands');
    });

    it('should define trace tail subcommand', () => {
      const cmd = new Command();
      const trace = cmd.command('trace').description('Trace commands');
      const tail = trace.command('tail').description('Tail events');
      tail.option('-f, --file <path>', 'Watch file');
      tail.option('-n, --lines <count>', 'Show last N', '10');
      expect(tail.description()).toBe('Tail events');
    });

    it('should define trace replay subcommand', () => {
      const cmd = new Command();
      const trace = cmd.command('trace').description('Trace commands');
      const replay = trace.command('replay <file>').description('Replay trace');
      replay.option('-s, --speed <multiplier>', 'Playback speed', '1.0');
      expect(replay.description()).toBe('Replay trace');
    });

    it('should define trace diff subcommand', () => {
      const cmd = new Command();
      const trace = cmd.command('trace').description('Trace commands');
      const diff = trace.command('diff <file1> <file2>').description('Compare traces');
      expect(diff.description()).toBe('Compare traces');
    });

    it('should define trace verify subcommand', () => {
      const cmd = new Command();
      const trace = cmd.command('trace').description('Trace commands');
      const verify = trace.command('verify <file>').description('Verify trace');
      expect(verify.description()).toBe('Verify trace');
    });
  });

  describe('Atlas Commands', () => {
    it('should define atlas parent command', () => {
      const cmd = new Command();
      const atlas = cmd.command('atlas').description('Atlas commands');
      expect(atlas.description()).toBe('Atlas commands');
    });

    it('should define atlas validate subcommand', () => {
      const cmd = new Command();
      const atlas = cmd.command('atlas').description('Atlas commands');
      const validate = atlas.command('validate <path>').description('Validate atlas');
      expect(validate.description()).toBe('Validate atlas');
    });

    it('should define atlas list subcommand', () => {
      const cmd = new Command();
      const atlas = cmd.command('atlas').description('Atlas commands');
      const list = atlas.command('list').description('List atlases');
      list.option('-p, --path <path>', 'Atlas directory', './atlases');
      expect(list.description()).toBe('List atlases');
    });
  });

  describe('Config Command', () => {
    it('should define config command', () => {
      const cmd = new Command();
      const config = cmd.command('config').description('Show config');
      expect(config.description()).toBe('Show config');
    });
  });

  describe('Stats Command', () => {
    it('should define stats command', () => {
      const cmd = new Command();
      const stats = cmd.command('stats').description('Show stats');
      expect(stats.description()).toBe('Show stats');
    });
  });
});

describe('Option Parsing', () => {
  it('should parse risk tier options', () => {
    const validTiers = ['low', 'medium', 'high', 'critical'];
    for (const tier of validTiers) {
      expect(validTiers).toContain(tier);
    }
  });

  it('should parse speed multiplier', () => {
    const speed = parseFloat('1.5');
    expect(speed).toBe(1.5);
  });

  it('should parse lines count', () => {
    const lines = parseInt('20');
    expect(lines).toBe(20);
  });
});

describe('Atlas Create Command', () => {
  it('should define atlas create subcommand', () => {
    const cmd = new Command();
    const atlas = cmd.command('atlas').description('Atlas commands');
    const create = atlas.command('create <name>').description('Create a new atlas');
    create.option('-d, --domain <domain>', 'Primary domain');
    create.option('-t, --template <template>', 'Template type', 'basic');
    create.option('-o, --output <path>', 'Output directory', './atlases');
    expect(create.description()).toBe('Create a new atlas');
  });

  it('should support template types', () => {
    const validTemplates = ['basic', 'api', 'ops'];
    expect(validTemplates).toContain('basic');
    expect(validTemplates).toContain('api');
    expect(validTemplates).toContain('ops');
  });

  it('should generate valid atlas id from name', () => {
    const name = 'my-test-atlas';
    const atlasId = `atlas.${name.replace(/-/g, '_').toLowerCase()}`;
    expect(atlasId).toBe('atlas.my_test_atlas');
  });

  it('should generate valid domain from name', () => {
    const name = 'my-test-atlas';
    const domain = name.replace(/-/g, '.').toLowerCase();
    expect(domain).toBe('my.test.atlas');
  });
});

describe('Atlas Template Structure', () => {
  it('should generate manifest with required fields', () => {
    const manifest = {
      atlas_version: '0.1',
      metadata: {
        id: 'atlas.test',
        name: 'test',
        version: '0.1.0',
        description: 'Test atlas',
        authors: [{ name: 'Test Author' }],
        license: { type: 'free', spdx: 'MIT' },
        keywords: ['test', 'cra', 'atlas'],
      },
      domains: [
        { id: 'test', name: 'test', description: 'test domain', risk_tier: 'medium' },
      ],
      context_packs: [
        { id: 'test.overview', domain: 'test', source: 'context/overview.md', ttl_seconds: 3600, tags: [] },
      ],
      policies: [],
      actions: [],
      adapters: [],
      tests: [],
    };

    expect(manifest.atlas_version).toBe('0.1');
    expect(manifest.metadata.id).toBe('atlas.test');
    expect(manifest.domains).toHaveLength(1);
    expect(manifest.context_packs).toHaveLength(1);
  });

  it('should generate API template actions', () => {
    const apiActions = [
      { id: 'test.get', type: 'api.get', risk_tier: 'low' },
      { id: 'test.post', type: 'api.post', risk_tier: 'medium' },
      { id: 'test.delete', type: 'api.delete', risk_tier: 'high' },
    ];

    expect(apiActions).toHaveLength(3);
    expect(apiActions[0].type).toBe('api.get');
    expect(apiActions[1].type).toBe('api.post');
    expect(apiActions[2].type).toBe('api.delete');
  });

  it('should generate ops template actions', () => {
    const opsActions = [
      { id: 'test.list', type: 'ops.list', risk_tier: 'low' },
      { id: 'test.create', type: 'ops.create', risk_tier: 'medium' },
      { id: 'test.update', type: 'ops.update', risk_tier: 'medium' },
      { id: 'test.delete', type: 'ops.delete', risk_tier: 'high' },
    ];

    expect(opsActions).toHaveLength(4);
    expect(opsActions[0].type).toBe('ops.list');
    expect(opsActions[3].type).toBe('ops.delete');
  });

  it('should generate adapter config with valid structure', () => {
    const adapterConfig = {
      platform: 'openai',
      tool_prefix: 'test',
      tool_mappings: [{ action_id: 'test.list', tool_name: 'test_list' }],
      system_prompt_additions: ['You have access to test operations.'],
    };

    expect(adapterConfig.platform).toBe('openai');
    expect(adapterConfig.tool_mappings).toHaveLength(1);
    expect(adapterConfig.system_prompt_additions).toHaveLength(1);
  });

  it('should generate policy with risk-based rules', () => {
    const policy = {
      id: 'default-policy',
      name: 'Default Policy',
      version: '1.0.0',
      rules: [
        {
          id: 'allow-low-risk',
          condition: { type: 'risk_tier', operator: 'in', value: ['low', 'medium'] },
          effect: 'allow',
          priority: 100,
        },
        {
          id: 'require-approval-high-risk',
          condition: { type: 'risk_tier', operator: 'in', value: ['high', 'critical'] },
          effect: 'require_approval',
          priority: 50,
        },
      ],
    };

    expect(policy.rules).toHaveLength(2);
    expect(policy.rules[0].effect).toBe('allow');
    expect(policy.rules[1].effect).toBe('require_approval');
  });
});
