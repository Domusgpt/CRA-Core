/**
 * OpenAI Platform Adapter
 *
 * Translates CRA artifacts to OpenAI tool calling format.
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
// OpenAI-Specific Types
// =============================================================================

export interface OpenAITool {
  type: 'function';
  function: {
    name: string;
    description: string;
    parameters: Record<string, unknown>;
  };
}

export interface OpenAIToolCall {
  id: string;
  type: 'function';
  function: {
    name: string;
    arguments: string; // JSON string
  };
}

// =============================================================================
// OpenAI Adapter
// =============================================================================

export class OpenAIAdapter extends BasePlatformAdapter {
  constructor(options: AdapterOptions) {
    super({ ...options, platform: 'openai' });
  }

  /**
   * Convert CARP actions to OpenAI tool definitions
   */
  toToolDefinitions(actions: ActionPermission[]): PlatformTool[] {
    return actions.map(action => ({
      name: this.toToolName(action.action_type),
      description: action.description,
      parameters: action.schema,
    }));
  }

  /**
   * Get OpenAI-specific tool format
   */
  toOpenAITools(actions: ActionPermission[]): OpenAITool[] {
    return actions.map(action => ({
      type: 'function' as const,
      function: {
        name: this.toToolName(action.action_type),
        description: action.description,
        parameters: action.schema,
      },
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
    }

    parts.push('## Context\n');
    parts.push('The following context has been provided to assist you:\n');

    for (const block of contextBlocks) {
      parts.push(`### ${block.domain}`);
      parts.push(block.content);
      parts.push('');
    }

    parts.push('\n## Guidelines');
    parts.push('- Only use the tools that have been provided');
    parts.push('- Follow the context guidelines for each domain');
    parts.push('- Request clarification if the task is unclear');

    if (options.suffix) {
      parts.push('');
      parts.push(options.suffix);
    }

    return parts.join('\n');
  }

  /**
   * Translate OpenAI tool call to CARP action
   */
  fromToolCall(call: PlatformToolCall): TranslatedAction {
    return {
      action_type: this.fromToolName(call.name),
      parameters: call.arguments,
    };
  }

  /**
   * Parse OpenAI tool call format
   */
  parseOpenAIToolCall(call: OpenAIToolCall): TranslatedAction {
    return {
      action_type: this.fromToolName(call.function.name),
      parameters: JSON.parse(call.function.arguments),
    };
  }

  /**
   * Convert action type to tool name (snake_case)
   */
  private toToolName(actionType: string): string {
    // api.github.create_issue -> github_create_issue
    return actionType.replace(/^api\./, '').replace(/\./g, '_');
  }

  /**
   * Convert tool name back to action type
   */
  private fromToolName(toolName: string): string {
    // github_create_issue -> api.github.create_issue
    const parts = toolName.split('_');
    if (parts.length >= 2) {
      return `api.${parts[0]}.${parts.slice(1).join('_')}`;
    }
    return `api.${toolName}`;
  }
}

// Register adapter
registerAdapter('openai', OpenAIAdapter);
