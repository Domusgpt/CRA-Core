/**
 * Platform Adapters Tests
 *
 * Tests for OpenAI and Claude adapters.
 */

import { describe, it, expect } from 'vitest';
import { OpenAIAdapter, OpenAITool, OpenAIToolCall } from '../openai.js';
import { ClaudeAdapter, ClaudeTool, ClaudeToolUse } from '../claude.js';
import { getAdapter, getSupportedPlatforms } from '../base.js';
import type { ActionPermission, ContextBlock } from '@cra/protocol';

// Sample test data
const sampleActions: ActionPermission[] = [
  {
    action_id: 'action-1',
    action_type: 'api.github.create_issue',
    name: 'Create Issue',
    description: 'Create a new GitHub issue',
    schema: {
      type: 'object',
      properties: {
        title: { type: 'string', description: 'Issue title' },
        body: { type: 'string', description: 'Issue body' },
        labels: { type: 'array', items: { type: 'string' } },
      },
      required: ['title'],
    },
    examples: [
      {
        description: 'Create a bug report',
        input: { title: 'Bug: Login fails', body: 'Steps to reproduce...' },
        expected_output: { issue_number: 123 },
      },
    ],
    constraints: [],
    requires_approval: false,
    risk_tier: 'low',
    atlas_ref: 'github-ops@1.0',
    evidence_refs: [],
    valid_until: new Date(Date.now() + 300000).toISOString(),
  },
  {
    action_id: 'action-2',
    action_type: 'api.github.delete_repo',
    name: 'Delete Repository',
    description: 'Delete a GitHub repository',
    schema: {
      type: 'object',
      properties: {
        owner: { type: 'string' },
        repo: { type: 'string' },
      },
      required: ['owner', 'repo'],
    },
    examples: [],
    constraints: [],
    requires_approval: true,
    risk_tier: 'critical',
    atlas_ref: 'github-ops@1.0',
    evidence_refs: [],
    valid_until: new Date(Date.now() + 300000).toISOString(),
  },
];

const sampleContextBlocks: ContextBlock[] = [
  {
    block_id: 'block-1',
    content_hash: 'abc123',
    atlas_ref: 'github-ops@1.0',
    pack_ref: 'github-basics',
    domain: 'github.api',
    content_type: 'markdown',
    content: '# GitHub API Guidelines\n\nUse the GitHub REST API v3.',
    token_count: 50,
    ttl_seconds: 300,
    tags: ['github', 'api'],
    priority: 100,
    evidence_refs: [],
  },
  {
    block_id: 'block-2',
    content_hash: 'def456',
    atlas_ref: 'github-ops@1.0',
    pack_ref: 'issue-templates',
    domain: 'github.issues',
    content_type: 'markdown',
    content: '# Issue Templates\n\nUse structured templates for bug reports.',
    token_count: 40,
    ttl_seconds: 300,
    tags: ['github', 'issues'],
    priority: 80,
    evidence_refs: [],
  },
];

describe('OpenAI Adapter', () => {
  const adapter = new OpenAIAdapter({ platform: 'openai' });

  describe('Tool Definitions', () => {
    it('should convert actions to generic tool definitions', () => {
      const tools = adapter.toToolDefinitions(sampleActions);

      expect(tools).toHaveLength(2);
      expect(tools[0].name).toBe('github_create_issue');
      expect(tools[0].description).toBe('Create a new GitHub issue');
      expect(tools[0].parameters).toEqual(sampleActions[0].schema);
    });

    it('should convert actions to OpenAI tool format', () => {
      const tools = adapter.toOpenAITools(sampleActions);

      expect(tools).toHaveLength(2);

      const tool = tools[0];
      expect(tool.type).toBe('function');
      expect(tool.function.name).toBe('github_create_issue');
      expect(tool.function.description).toBe('Create a new GitHub issue');
      expect(tool.function.parameters).toBeDefined();
    });

    it('should handle action type to tool name conversion', () => {
      const tools = adapter.toOpenAITools(sampleActions);

      expect(tools[0].function.name).toBe('github_create_issue');
      expect(tools[1].function.name).toBe('github_delete_repo');
    });
  });

  describe('System Prompt', () => {
    it('should convert context blocks to system prompt', () => {
      const prompt = adapter.toSystemPrompt(sampleContextBlocks);

      expect(prompt).toContain('## Context');
      expect(prompt).toContain('github.api');
      expect(prompt).toContain('GitHub API Guidelines');
      expect(prompt).toContain('github.issues');
      expect(prompt).toContain('Issue Templates');
      expect(prompt).toContain('## Guidelines');
    });

    it('should include prefix and suffix', () => {
      const prompt = adapter.toSystemPrompt(sampleContextBlocks, {
        prefix: 'You are a helpful assistant.',
        suffix: 'Always be thorough.',
      });

      expect(prompt).toMatch(/^You are a helpful assistant/);
      expect(prompt).toMatch(/Always be thorough\.$/);
    });

    it('should include guidelines', () => {
      const prompt = adapter.toSystemPrompt(sampleContextBlocks);

      expect(prompt).toContain('Only use the tools that have been provided');
      expect(prompt).toContain('Request clarification');
    });
  });

  describe('Tool Call Translation', () => {
    it('should translate generic tool call to CARP action', () => {
      const toolCall = {
        name: 'github_create_issue',
        arguments: { title: 'Bug', body: 'Description' },
      };

      const action = adapter.fromToolCall(toolCall);

      expect(action.action_type).toBe('api.github.create_issue');
      expect(action.parameters).toEqual({ title: 'Bug', body: 'Description' });
    });

    it('should parse OpenAI tool call format', () => {
      const openAICall: OpenAIToolCall = {
        id: 'call_123',
        type: 'function',
        function: {
          name: 'github_create_issue',
          arguments: '{"title": "Bug", "body": "Description"}',
        },
      };

      const action = adapter.parseOpenAIToolCall(openAICall);

      expect(action.action_type).toBe('api.github.create_issue');
      expect(action.parameters).toEqual({ title: 'Bug', body: 'Description' });
    });
  });

  describe('Full Configuration', () => {
    it('should get full platform configuration', () => {
      // Create a mock resolution
      const mockResolution = {
        carp_version: '1.0' as const,
        request_id: 'req-1',
        resolution_id: 'res-1',
        timestamp: new Date().toISOString(),
        decision: { type: 'allow' as const },
        context_blocks: sampleContextBlocks,
        allowed_actions: sampleActions,
        denied_actions: [],
        policies_applied: [],
        evidence: [],
        ttl: {
          context_expires_at: new Date().toISOString(),
          resolution_expires_at: new Date().toISOString(),
          refresh_hint_seconds: 240,
        },
        telemetry_link: {
          trace_id: 'trace-1',
          span_id: 'span-1',
          events_emitted: 0,
        },
      };

      const config = adapter.getConfiguration(mockResolution);

      expect(config.systemPrompt).toContain('github.api');
      expect(config.tools).toHaveLength(2);
      expect(config.tools[0].name).toBe('github_create_issue');
    });
  });
});

describe('Claude Adapter', () => {
  const adapter = new ClaudeAdapter({ platform: 'claude' });

  describe('Tool Definitions', () => {
    it('should convert actions to Claude tool format', () => {
      const tools = adapter.toClaudeTools(sampleActions);

      expect(tools).toHaveLength(2);

      const tool = tools[0];
      expect(tool.name).toBe('github_create_issue');
      expect(tool.description).toContain('Create a new GitHub issue');
      expect(tool.input_schema).toBeDefined();
    });

    it('should enhance description for high risk actions', () => {
      const tools = adapter.toClaudeTools(sampleActions);

      const criticalTool = tools[1]; // delete_repo is critical
      expect(criticalTool.description).toContain('CRITICAL RISK');
      expect(criticalTool.description).toContain('requires explicit approval');
    });

    it('should include example in description', () => {
      const tools = adapter.toClaudeTools(sampleActions);

      const toolWithExample = tools[0]; // create_issue has example
      expect(toolWithExample.description).toContain('Example:');
      expect(toolWithExample.description).toContain('Create a bug report');
    });
  });

  describe('System Prompt', () => {
    it('should convert context blocks to Claude-style prompt', () => {
      const prompt = adapter.toSystemPrompt(sampleContextBlocks);

      expect(prompt).toContain('<cra_governance>');
      expect(prompt).toContain('</cra_governance>');
      expect(prompt).toContain('<context>');
      expect(prompt).toContain('</context>');
      expect(prompt).toContain('<domain name="github.api">');
      expect(prompt).toContain('</domain>');
      expect(prompt).toContain('<guidelines>');
      expect(prompt).toContain('</guidelines>');
    });

    it('should include audit notice', () => {
      const prompt = adapter.toSystemPrompt(sampleContextBlocks);

      expect(prompt).toContain('logged for audit purposes');
    });

    it('should include prefix and suffix', () => {
      const prompt = adapter.toSystemPrompt(sampleContextBlocks, {
        prefix: 'System prefix here.',
        suffix: 'System suffix here.',
      });

      expect(prompt).toContain('System prefix here.');
      expect(prompt).toContain('System suffix here.');
    });
  });

  describe('Tool Use Translation', () => {
    it('should translate generic tool call', () => {
      const toolCall = {
        name: 'github_create_issue',
        arguments: { title: 'Bug', body: 'Description' },
      };

      const action = adapter.fromToolCall(toolCall);

      expect(action.action_type).toBe('api.github.create_issue');
      expect(action.parameters).toEqual({ title: 'Bug', body: 'Description' });
    });

    it('should parse Claude tool use block', () => {
      const toolUse: ClaudeToolUse = {
        type: 'tool_use',
        id: 'toolu_123',
        name: 'github_create_issue',
        input: { title: 'Bug', body: 'Description' },
      };

      const action = adapter.parseClaudeToolUse(toolUse);

      expect(action.action_type).toBe('api.github.create_issue');
      expect(action.parameters).toEqual({ title: 'Bug', body: 'Description' });
    });
  });

  describe('Tool Results', () => {
    it('should create tool result from string', () => {
      const result = adapter.createToolResult('toolu_123', 'Success!');

      expect(result.type).toBe('tool_result');
      expect(result.tool_use_id).toBe('toolu_123');
      expect(result.content).toBe('Success!');
      expect(result.is_error).toBe(false);
    });

    it('should create tool result from object', () => {
      const result = adapter.createToolResult('toolu_123', { issue_number: 42 });

      expect(result.content).toContain('"issue_number": 42');
    });

    it('should create error tool result', () => {
      const result = adapter.createToolResult('toolu_123', 'Failed', true);

      expect(result.is_error).toBe(true);
    });
  });
});

describe('Adapter Registry', () => {
  it('should get OpenAI adapter', () => {
    const adapter = getAdapter('openai');
    expect(adapter).toBeInstanceOf(OpenAIAdapter);
  });

  it('should get Claude adapter', () => {
    const adapter = getAdapter('claude');
    expect(adapter).toBeInstanceOf(ClaudeAdapter);
  });

  it('should throw for unknown platform', () => {
    expect(() => getAdapter('unknown' as any)).toThrow('No adapter registered');
  });

  it('should list supported platforms', () => {
    const platforms = getSupportedPlatforms();
    expect(platforms).toContain('openai');
    expect(platforms).toContain('claude');
  });
});

describe('Name Conversion', () => {
  const openAIAdapter = new OpenAIAdapter({ platform: 'openai' });
  const claudeAdapter = new ClaudeAdapter({ platform: 'claude' });

  const testCases = [
    { actionType: 'api.github.create_issue', toolName: 'github_create_issue' },
    { actionType: 'api.slack.send_message', toolName: 'slack_send_message' },
    { actionType: 'api.aws.s3.upload', toolName: 'aws_s3_upload' },
  ];

  for (const { actionType, toolName } of testCases) {
    it(`should convert ${actionType} to ${toolName}`, () => {
      const action: ActionPermission = {
        action_id: 'test',
        action_type: actionType,
        name: 'Test',
        description: 'Test action',
        schema: {},
        examples: [],
        constraints: [],
        requires_approval: false,
        risk_tier: 'low',
        atlas_ref: 'test@1.0',
        evidence_refs: [],
        valid_until: new Date().toISOString(),
      };

      const tools = openAIAdapter.toOpenAITools([action]);
      expect(tools[0].function.name).toBe(toolName);
    });

    it(`should convert ${toolName} back to api.* format`, () => {
      const result = openAIAdapter.fromToolCall({
        name: toolName,
        arguments: {},
      });

      // The conversion adds api. prefix
      expect(result.action_type).toMatch(/^api\./);
    });
  }
});
