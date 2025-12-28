/**
 * MCP Server Factory
 *
 * Convenience function for creating CRA MCP servers.
 */

import { CRAMCPServer, type MCPServerConfig } from './server.js';

/**
 * Create and start a CRA MCP server
 */
export async function createMCPServer(
  config: MCPServerConfig = {}
): Promise<CRAMCPServer> {
  const server = new CRAMCPServer(config);
  await server.start();
  return server;
}
