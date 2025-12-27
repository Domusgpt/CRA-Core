/**
 * CRA Platform Adapters
 *
 * Translate CRA artifacts to platform-specific formats.
 * Supports OpenAI, Claude, Google ADK, and MCP.
 */

// Base adapter and registry
export {
  BasePlatformAdapter,
  registerAdapter,
  getAdapter,
  getSupportedPlatforms,
} from './base.js';
export type {
  PlatformTool,
  PlatformToolCall,
  TranslatedAction,
  AdapterOptions,
} from './base.js';

// OpenAI adapter
export { OpenAIAdapter } from './openai.js';
export type { OpenAITool, OpenAIToolCall } from './openai.js';

// Claude adapter
export { ClaudeAdapter } from './claude.js';
export type { ClaudeTool, ClaudeToolUse, ClaudeToolResult } from './claude.js';

// MCP adapter
export { MCPAdapter } from './mcp.js';
export type {
  MCPTool,
  MCPResource,
  MCPToolCallRequest,
  MCPToolCallResponse,
  MCPResourceReadRequest,
  MCPResourceReadResponse,
} from './mcp.js';

// Import all adapters to register them
import './openai.js';
import './claude.js';
import './mcp.js';
