/**
 * Claude Platform Adapter
 *
 * Translates CRA artifacts to Anthropic Claude tool use format.
 */

import type { ActionPermission, ContextBlock } from '@cra/protocol';
import {
  BasePlatformAdapter,
  PlatformTool,
  PlatformToolCall,
  TranslatedAction,
  AdapterOptions,
  registerAdapter,
} from './base.js';

// =============================================================================
// Claude-Specific Types
// =============================================================================

export interface ClaudeTool {
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
}

export interface ClaudeToolUse {
  type: 'tool_use';
  id: string;
  name: string;
  input: Record<string, unknown>;
}

export interface ClaudeToolResult {
  type: 'tool_result';
  tool_use_id: string;
  content: string | { type: 'text'; text: string }[];
  is_error?: boolean;
}

// =============================================================================
// Claude Adapter
// =============================================================================

export class ClaudeAdapter extends BasePlatformAdapter {
  constructor(options: AdapterOptions) {
    super({ ...options, platform: 'claude' });
  }

  /**
   * Convert CARP actions to generic tool definitions
   */
  toToolDefinitions(actions: ActionPermission[]): PlatformTool[] {
    return actions.map(action => ({
      name: this.toToolName(action.action_type),
      description: action.description,
      parameters: action.schema,
    }));
  }

  /**
   * Get Claude-specific tool format
   */
  toClaudeTools(actions: ActionPermission[]): ClaudeTool[] {
    return actions.map(action => ({
      name: this.toToolName(action.action_type),
      description: this.enhanceDescription(action),
      input_schema: action.schema,
    }));
  }

  /**
   * Convert context blocks to system prompt
   */
  toSystemPrompt(
    contextBlocks: ContextBlock[],
    options: { prefix?: string; suffix?: string } = {}
  ): string {
    const parts: string[] = [];

    if (options.prefix) {
      parts.push(options.prefix);
      parts.push('');
    }

    // Add CRA governance notice
    parts.push('<cra_governance>');
    parts.push('You are operating under CRA (Context Registry Agents) governance.');
    parts.push('The following context and tools have been authorized for this session.');
    parts.push('</cra_governance>');
    parts.push('');

    // Add context blocks
    parts.push('<context>');
    for (const block of contextBlocks) {
      parts.push(`<domain name="${block.domain}">`);
      parts.push(block.content);
      parts.push('</domain>');
      parts.push('');
    }
    parts.push('</context>');
    parts.push('');

    // Add guidelines
    parts.push('<guidelines>');
    parts.push('- Only use tools that have been explicitly provided');
    parts.push('- Follow the domain-specific context when using each tool');
    parts.push('- If uncertain, ask for clarification before proceeding');
    parts.push('- All tool usage is logged for audit purposes');
    parts.push('</guidelines>');

    if (options.suffix) {
      parts.push('');
      parts.push(options.suffix);
    }

    return parts.join('\n');
  }

  /**
   * Translate Claude tool use to CARP action
   */
  fromToolCall(call: PlatformToolCall): TranslatedAction {
    return {
      action_type: this.fromToolName(call.name),
      parameters: call.arguments,
    };
  }

  /**
   * Parse Claude tool use block
   */
  parseClaudeToolUse(toolUse: ClaudeToolUse): TranslatedAction {
    return {
      action_type: this.fromToolName(toolUse.name),
      parameters: toolUse.input,
    };
  }

  /**
   * Create a tool result for Claude
   */
  createToolResult(
    toolUseId: string,
    result: unknown,
    isError = false
  ): ClaudeToolResult {
    const content = typeof result === 'string' ? result : JSON.stringify(result, null, 2);
    return {
      type: 'tool_result',
      tool_use_id: toolUseId,
      content,
      is_error: isError,
    };
  }

  /**
   * Convert action type to tool name
   */
  private toToolName(actionType: string): string {
    // api.github.create_issue -> github_create_issue
    return actionType.replace(/^api\./, '').replace(/\./g, '_');
  }

  /**
   * Convert tool name back to action type
   */
  private fromToolName(toolName: string): string {
    const parts = toolName.split('_');
    if (parts.length >= 2) {
      return `api.${parts[0]}.${parts.slice(1).join('_')}`;
    }
    return `api.${toolName}`;
  }

  /**
   * Enhance tool description with examples and constraints
   */
  private enhanceDescription(action: ActionPermission): string {
    let desc = action.description;

    // Add risk indicator
    if (action.risk_tier === 'high' || action.risk_tier === 'critical') {
      desc = `⚠️ [${action.risk_tier.toUpperCase()} RISK] ${desc}`;
    }

    // Add example if available
    if (action.examples?.length) {
      const example = action.examples[0];
      desc += `\n\nExample: ${example.description}`;
    }

    // Add approval notice
    if (action.requires_approval) {
      desc += '\n\nNote: This action requires explicit approval.';
    }

    return desc;
  }
}

// Register adapter
registerAdapter('claude', ClaudeAdapter);
