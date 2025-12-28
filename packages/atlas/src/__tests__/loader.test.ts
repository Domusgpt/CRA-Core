/**
 * Atlas Loader Tests
 *
 * Tests for atlas loading, validation, context extraction, and policy evaluation.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { AtlasLoader } from '../loader.js';
import type { AtlasManifest } from '../types.js';

describe('AtlasLoader', () => {
  let loader: AtlasLoader;
  const testAtlasDir = './test-atlas-loader';

  // Helper to create a test atlas
  async function createTestAtlas(manifest: AtlasManifest, contextFiles: Record<string, string> = {}) {
    await fs.promises.mkdir(testAtlasDir, { recursive: true });
    await fs.promises.mkdir(path.join(testAtlasDir, 'context'), { recursive: true });

    await fs.promises.writeFile(
      path.join(testAtlasDir, 'atlas.json'),
      JSON.stringify(manifest, null, 2)
    );

    for (const [filename, content] of Object.entries(contextFiles)) {
      const filePath = path.join(testAtlasDir, filename);
      await fs.promises.mkdir(path.dirname(filePath), { recursive: true });
      await fs.promises.writeFile(filePath, content);
    }
  }

  // Minimal valid manifest
  const minimalManifest: AtlasManifest = {
    atlas_version: '0.1',
    metadata: {
      id: 'test-atlas',
      version: '1.0.0',
      name: 'Test Atlas',
      description: 'A test atlas',
    },
    domains: [
      {
        id: 'test.domain',
        name: 'Test Domain',
        description: 'A test domain',
      },
    ],
    context_packs: [],
    actions: [],
    policies: [],
    adapters: [],
  };

  beforeEach(() => {
    loader = new AtlasLoader({ validate: true });
  });

  afterEach(async () => {
    loader.clearCache();
    try {
      await fs.promises.rm(testAtlasDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('Loading', () => {
    it('should load a valid atlas from directory', async () => {
      await createTestAtlas(minimalManifest);

      const atlas = await loader.load(testAtlasDir);

      expect(atlas.ref).toBe('test-atlas@1.0.0');
      expect(atlas.manifest.metadata.id).toBe('test-atlas');
      expect(atlas.manifest.domains).toHaveLength(1);
    });

    it('should load atlas from YAML file', async () => {
      await fs.promises.mkdir(testAtlasDir, { recursive: true });
      await fs.promises.writeFile(
        path.join(testAtlasDir, 'atlas.yaml'),
        `atlas_version: "0.1"
metadata:
  id: yaml-atlas
  version: "1.0.0"
  name: YAML Atlas
  description: An atlas in YAML format
domains:
  - id: yaml.domain
    name: YAML Domain
    description: A YAML domain
context_packs: []
actions: []
policies: []
adapters: []`
      );

      const atlas = await loader.load(testAtlasDir);

      expect(atlas.ref).toBe('yaml-atlas@1.0.0');
      expect(atlas.manifest.domains[0].id).toBe('yaml.domain');
    });

    it('should throw error for missing atlas file', async () => {
      await fs.promises.mkdir(testAtlasDir, { recursive: true });

      await expect(loader.load(testAtlasDir)).rejects.toThrow('No atlas.json or atlas.yaml found');
    });

    it('should cache loaded atlases', async () => {
      await createTestAtlas(minimalManifest);

      const atlas1 = await loader.load(testAtlasDir);
      const atlas2 = await loader.load(testAtlasDir);

      // Should return cached instance
      expect(atlas1).toBe(atlas2);
    });

    it('should load context pack content', async () => {
      const manifestWithContext: AtlasManifest = {
        ...minimalManifest,
        context_packs: [
          {
            id: 'test-context',
            name: 'Test Context',
            domain: 'test.domain',
            source: 'context/test.md',
            format: 'markdown',
            priority: 100,
            tags: ['test'],
            evidence_sources: [],
          },
        ],
      };

      await createTestAtlas(manifestWithContext, {
        'context/test.md': '# Test Context\n\nThis is test content.',
      });

      const atlas = await loader.load(testAtlasDir);

      expect(atlas.context_content.get('test-context')).toBe('# Test Context\n\nThis is test content.');
    });
  });

  describe('Validation', () => {
    it('should validate a correct manifest', () => {
      const result = loader.validate(minimalManifest);

      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should reject invalid atlas version', () => {
      const invalid = { ...minimalManifest, atlas_version: '2.0' };

      const result = loader.validate(invalid as AtlasManifest);

      expect(result.valid).toBe(false);
      expect(result.errors.some(e => e.message.includes('version'))).toBe(true);
    });

    it('should reject missing metadata.id', () => {
      const invalid = {
        ...minimalManifest,
        metadata: { ...minimalManifest.metadata, id: '' },
      };

      const result = loader.validate(invalid as AtlasManifest);

      expect(result.valid).toBe(false);
      expect(result.errors.some(e => e.path === 'metadata.id')).toBe(true);
    });

    it('should reject duplicate domain IDs', () => {
      const invalid: AtlasManifest = {
        ...minimalManifest,
        domains: [
          { id: 'duplicate', name: 'First', description: 'First domain' },
          { id: 'duplicate', name: 'Second', description: 'Second domain' },
        ],
      };

      const result = loader.validate(invalid);

      expect(result.valid).toBe(false);
      expect(result.errors.some(e => e.message.includes('Duplicate domain'))).toBe(true);
    });

    it('should reject context pack with unknown domain', () => {
      const invalid: AtlasManifest = {
        ...minimalManifest,
        context_packs: [
          {
            id: 'orphan-context',
            name: 'Orphan',
            domain: 'nonexistent.domain',
            source: 'context/orphan.md',
            format: 'markdown',
            priority: 0,
            tags: [],
            evidence_sources: [],
          },
        ],
      };

      const result = loader.validate(invalid);

      expect(result.valid).toBe(false);
      expect(result.errors.some(e => e.message.includes('Unknown domain'))).toBe(true);
    });

    it('should reject action with unknown domain', () => {
      const invalid: AtlasManifest = {
        ...minimalManifest,
        actions: [
          {
            id: 'orphan-action',
            type: 'test.action',
            name: 'Orphan Action',
            description: 'An action without a domain',
            domain: 'nonexistent.domain',
            risk_tier: 'low',
            schema: {},
            examples: [],
          },
        ],
      };

      const result = loader.validate(invalid);

      expect(result.valid).toBe(false);
      expect(result.errors.some(e => e.message.includes('Unknown domain'))).toBe(true);
    });
  });

  describe('Context Blocks', () => {
    const manifestWithContext: AtlasManifest = {
      ...minimalManifest,
      domains: [
        { id: 'domain.a', name: 'Domain A', description: 'First domain' },
        { id: 'domain.b', name: 'Domain B', description: 'Second domain' },
      ],
      context_packs: [
        {
          id: 'context-a1',
          name: 'Context A1',
          domain: 'domain.a',
          source: 'context/a1.md',
          format: 'markdown',
          priority: 100,
          tags: ['important'],
          evidence_sources: [],
        },
        {
          id: 'context-a2',
          name: 'Context A2',
          domain: 'domain.a',
          source: 'context/a2.md',
          format: 'markdown',
          priority: 50,
          tags: ['optional'],
          evidence_sources: [],
        },
        {
          id: 'context-b1',
          name: 'Context B1',
          domain: 'domain.b',
          source: 'context/b1.md',
          format: 'markdown',
          priority: 75,
          tags: ['important'],
          evidence_sources: [],
        },
      ],
    };

    beforeEach(async () => {
      await createTestAtlas(manifestWithContext, {
        'context/a1.md': 'Content A1 - high priority',
        'context/a2.md': 'Content A2 - low priority',
        'context/b1.md': 'Content B1 - medium priority',
      });
    });

    it('should get all context blocks', async () => {
      const atlas = await loader.load(testAtlasDir);
      const blocks = loader.getContextBlocks(atlas);

      expect(blocks).toHaveLength(3);
    });

    it('should filter by domain', async () => {
      const atlas = await loader.load(testAtlasDir);
      const blocks = loader.getContextBlocks(atlas, { domains: ['domain.a'] });

      expect(blocks).toHaveLength(2);
      expect(blocks.every(b => b.domain === 'domain.a')).toBe(true);
    });

    it('should filter by tags', async () => {
      const atlas = await loader.load(testAtlasDir);
      const blocks = loader.getContextBlocks(atlas, { tags: ['important'] });

      expect(blocks).toHaveLength(2);
    });

    it('should respect max_tokens limit', async () => {
      const atlas = await loader.load(testAtlasDir);
      // Set very low limit to only get one block
      const blocks = loader.getContextBlocks(atlas, { max_tokens: 10 });

      expect(blocks.length).toBeLessThanOrEqual(2);
    });

    it('should sort by priority (descending)', async () => {
      const atlas = await loader.load(testAtlasDir);
      const blocks = loader.getContextBlocks(atlas);

      // Higher priority should come first
      expect(blocks[0].priority).toBeGreaterThanOrEqual(blocks[1].priority);
      expect(blocks[1].priority).toBeGreaterThanOrEqual(blocks[2].priority);
    });

    it('should include token count and hash', async () => {
      const atlas = await loader.load(testAtlasDir);
      const blocks = loader.getContextBlocks(atlas);

      for (const block of blocks) {
        expect(block.token_count).toBeGreaterThan(0);
        expect(block.content_hash).toMatch(/^[a-f0-9]{64}$/);
        expect(block.block_id).toMatch(/^[0-9a-f-]{36}$/);
      }
    });
  });

  describe('Action Permissions', () => {
    const manifestWithActions: AtlasManifest = {
      ...minimalManifest,
      actions: [
        {
          id: 'action.low',
          type: 'test.action.low',
          name: 'Low Risk Action',
          description: 'A low risk action',
          domain: 'test.domain',
          risk_tier: 'low',
          schema: { type: 'object' },
          examples: [],
        },
        {
          id: 'action.medium',
          type: 'test.action.medium',
          name: 'Medium Risk Action',
          description: 'A medium risk action',
          domain: 'test.domain',
          risk_tier: 'medium',
          schema: { type: 'object' },
          examples: [],
        },
        {
          id: 'action.high',
          type: 'test.action.high',
          name: 'High Risk Action',
          description: 'A high risk action',
          domain: 'test.domain',
          risk_tier: 'high',
          schema: { type: 'object' },
          examples: [],
        },
      ],
    };

    it('should get all action permissions', async () => {
      await createTestAtlas(manifestWithActions);
      const atlas = await loader.load(testAtlasDir);
      const permissions = loader.getActionPermissions(atlas);

      expect(permissions).toHaveLength(3);
    });

    it('should filter by risk tier', async () => {
      await createTestAtlas(manifestWithActions);
      const atlas = await loader.load(testAtlasDir);

      // Low risk should only get low actions
      const lowPerms = loader.getActionPermissions(atlas, { risk_tier: 'low' });
      expect(lowPerms).toHaveLength(1);
      expect(lowPerms[0].risk_tier).toBe('low');

      // Medium risk should get low and medium actions
      const mediumPerms = loader.getActionPermissions(atlas, { risk_tier: 'medium' });
      expect(mediumPerms).toHaveLength(2);

      // High risk should get all actions
      const highPerms = loader.getActionPermissions(atlas, { risk_tier: 'high' });
      expect(highPerms).toHaveLength(3);
    });

    it('should mark high risk actions as requiring approval', async () => {
      await createTestAtlas(manifestWithActions);
      const atlas = await loader.load(testAtlasDir);
      const permissions = loader.getActionPermissions(atlas);

      const highRiskPerm = permissions.find(p => p.risk_tier === 'high');
      expect(highRiskPerm?.requires_approval).toBe(true);

      const lowRiskPerm = permissions.find(p => p.risk_tier === 'low');
      expect(lowRiskPerm?.requires_approval).toBe(false);
    });

    it('should include valid_until timestamp', async () => {
      await createTestAtlas(manifestWithActions);
      const atlas = await loader.load(testAtlasDir);
      const permissions = loader.getActionPermissions(atlas);

      for (const perm of permissions) {
        expect(perm.valid_until).toMatch(/^\d{4}-\d{2}-\d{2}T/);
        expect(new Date(perm.valid_until).getTime()).toBeGreaterThan(Date.now());
      }
    });
  });

  describe('Policy Evaluation', () => {
    const manifestWithPolicies: AtlasManifest = {
      ...minimalManifest,
      policies: [
        {
          id: 'risk-policy',
          name: 'Risk Policy',
          version: '1.0',
          rules: [
            {
              id: 'deny-critical',
              name: 'Deny Critical Risk',
              priority: 100,
              condition: {
                type: 'risk_tier',
                operator: 'eq',
                value: 'critical',
              },
              effect: 'deny',
              message: 'Critical risk actions are not allowed',
            },
            {
              id: 'approve-high',
              name: 'Require Approval for High Risk',
              priority: 90,
              condition: {
                type: 'risk_tier',
                operator: 'eq',
                value: 'high',
              },
              effect: 'require_approval',
            },
            {
              id: 'allow-low',
              name: 'Allow Low Risk',
              priority: 80,
              condition: {
                type: 'risk_tier',
                operator: 'eq',
                value: 'low',
              },
              effect: 'allow',
            },
          ],
        },
      ],
    };

    it('should deny critical risk', async () => {
      await createTestAtlas(manifestWithPolicies);
      const atlas = await loader.load(testAtlasDir);

      const result = loader.evaluatePolicies(atlas, { risk_tier: 'critical' });

      expect(result.allowed).toBe(false);
      expect(result.matched_rules.some(r => r.effect === 'deny')).toBe(true);
    });

    it('should require approval for high risk', async () => {
      await createTestAtlas(manifestWithPolicies);
      const atlas = await loader.load(testAtlasDir);

      const result = loader.evaluatePolicies(atlas, { risk_tier: 'high' });

      expect(result.requires_approval).toBe(true);
    });

    it('should allow low risk without approval', async () => {
      await createTestAtlas(manifestWithPolicies);
      const atlas = await loader.load(testAtlasDir);

      const result = loader.evaluatePolicies(atlas, { risk_tier: 'low' });

      expect(result.allowed).toBe(true);
      expect(result.requires_approval).toBe(false);
    });

    it('should track matched rules', async () => {
      await createTestAtlas(manifestWithPolicies);
      const atlas = await loader.load(testAtlasDir);

      const result = loader.evaluatePolicies(atlas, { risk_tier: 'high' });

      expect(result.matched_rules).toHaveLength(1);
      expect(result.matched_rules[0].policy_id).toBe('risk-policy');
      expect(result.matched_rules[0].rule_id).toBe('approve-high');
    });
  });

  describe('Cache Management', () => {
    it('should clear cache', async () => {
      await createTestAtlas(minimalManifest);

      const atlas1 = await loader.load(testAtlasDir);
      loader.clearCache();
      const atlas2 = await loader.load(testAtlasDir);

      // After clearing, should be different instances
      expect(atlas1).not.toBe(atlas2);
    });

    it('should prune expired cache entries', async () => {
      // Create loader with very short TTL
      const shortTtlLoader = new AtlasLoader({
        validate: true,
        cache_ttl_ms: 1, // 1ms TTL
      });

      await createTestAtlas(minimalManifest);
      await shortTtlLoader.load(testAtlasDir);

      // Wait for cache to expire
      await new Promise(resolve => setTimeout(resolve, 10));

      const pruned = shortTtlLoader.pruneCache();
      expect(pruned).toBe(1);
    });
  });
});
