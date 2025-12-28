/**
 * CRA MCP Server
 *
 * Exposes CRA capabilities via the Model Context Protocol (MCP).
 * This allows any MCP-compatible client to use CRA for context
 * resolution and action execution.
 */

export { CRAMCPServer, type MCPServerConfig } from './server.js';
export { createMCPServer } from './factory.js';
