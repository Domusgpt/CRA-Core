/**
 * MCP (Model Context Protocol) Platform Adapter
 *
 * Translates CRA artifacts to MCP server format.
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
// MCP-Specific Types
// =============================================================================

export interface MCPTool {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
}

export interface MCPResource {
  uri: string;
  name: string;
  description?: string;
  mimeType?: string;
}

export interface MCPToolCallRequest {
  method: 'tools/call';
  params: {
    name: string;
    arguments: Record<string, unknown>;
  };
}

export interface MCPToolCallResponse {
  content: {
    type: 'text';
    text: string;
  }[];
  isError?: boolean;
}

export interface MCPResourceReadRequest {
  method: 'resources/read';
  params: {
    uri: string;
  };
}

export interface MCPResourceReadResponse {
  contents: {
    uri: string;
    mimeType?: string;
    text?: string;
    blob?: string;
  }[];
}

// =============================================================================
// MCP Adapter
// =============================================================================

export class MCPAdapter extends BasePlatformAdapter {
  constructor(options: AdapterOptions) {
    super({ ...options, platform: 'mcp' });
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
   * Get MCP-specific tool format
   */
  toMCPTools(actions: ActionPermission[]): MCPTool[] {
    return actions.map(action => ({
      name: this.toToolName(action.action_type),
      description: action.description,
      inputSchema: action.schema,
    }));
  }

  /**
   * Convert context blocks to MCP resources
   */
  toMCPResources(contextBlocks: ContextBlock[]): MCPResource[] {
    return contextBlocks.map(block => ({
      uri: `cra://${block.atlas_ref}/${block.pack_ref}`,
      name: block.pack_ref,
      description: `Context for ${block.domain}`,
      mimeType: this.getMimeType(block.content_type),
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

    parts.push('# CRA Context');
    parts.push('');

    for (const block of contextBlocks) {
      parts.push(`## ${block.domain}`);
      parts.push('');
      parts.push(block.content);
      parts.push('');
    }

    if (options.suffix) {
      parts.push(options.suffix);
    }

    return parts.join('\n');
  }

  /**
   * Translate MCP tool call to CARP action
   */
  fromToolCall(call: PlatformToolCall): TranslatedAction {
    return {
      action_type: this.fromToolName(call.name),
      parameters: call.arguments,
    };
  }

  /**
   * Parse MCP tool call request
   */
  parseMCPToolCall(request: MCPToolCallRequest): TranslatedAction {
    return {
      action_type: this.fromToolName(request.params.name),
      parameters: request.params.arguments,
    };
  }

  /**
   * Create MCP tool call response
   */
  createMCPResponse(result: unknown, isError = false): MCPToolCallResponse {
    const text = typeof result === 'string' ? result : JSON.stringify(result, null, 2);
    return {
      content: [{ type: 'text', text }],
      isError,
    };
  }

  /**
   * Get resource content for MCP resource read
   */
  getResourceContent(
    uri: string,
    contextBlocks: ContextBlock[]
  ): MCPResourceReadResponse | null {
    // Parse URI: cra://atlas_ref/pack_ref
    const match = uri.match(/^cra:\/\/([^/]+)\/(.+)$/);
    if (!match) return null;

    const [, atlasRef, packRef] = match;
    const block = contextBlocks.find(
      b => b.atlas_ref === atlasRef && b.pack_ref === packRef
    );

    if (!block) return null;

    return {
      contents: [{
        uri,
        mimeType: this.getMimeType(block.content_type),
        text: block.content,
      }],
    };
  }

  /**
   * Generate MCP server capabilities
   */
  getServerCapabilities(actions: ActionPermission[], contextBlocks: ContextBlock[]): {
    capabilities: {
      tools?: { listChanged?: boolean };
      resources?: { subscribe?: boolean; listChanged?: boolean };
      prompts?: { listChanged?: boolean };
    };
    serverInfo: {
      name: string;
      version: string;
    };
  } {
    return {
      capabilities: {
        tools: actions.length > 0 ? { listChanged: false } : undefined,
        resources: contextBlocks.length > 0 ? { subscribe: false, listChanged: false } : undefined,
      },
      serverInfo: {
        name: 'cra-mcp-server',
        version: '0.1.0',
      },
    };
  }

  /**
   * Convert action type to tool name
   */
  private toToolName(actionType: string): string {
    // api.github.create_issue -> create_issue
    const parts = actionType.split('.');
    return parts[parts.length - 1];
  }

  /**
   * Convert tool name back to action type
   */
  private fromToolName(toolName: string): string {
    // This is a simplified mapping; in practice, you'd use
    // the tool mappings from the adapter config
    return `api.${toolName}`;
  }

  /**
   * Get MIME type for content type
   */
  private getMimeType(contentType: string): string {
    switch (contentType) {
      case 'markdown':
        return 'text/markdown';
      case 'json':
        return 'application/json';
      case 'yaml':
        return 'application/yaml';
      default:
        return 'text/plain';
    }
  }
}

// Register adapter
registerAdapter('mcp', MCPAdapter);
