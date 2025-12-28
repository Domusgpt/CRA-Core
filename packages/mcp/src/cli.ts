#!/usr/bin/env node

/**
 * CRA MCP Server CLI
 *
 * Run the CRA MCP server from command line.
 * Used by Claude Desktop and other MCP clients.
 */

import { CRAMCPServer } from './server.js';

async function main(): Promise<void> {
  // Parse command line arguments
  const args = process.argv.slice(2);
  const debug = args.includes('--debug') || args.includes('-d');
  const atlasPaths: string[] = [];

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--atlas' || args[i] === '-a') {
      if (args[i + 1]) {
        atlasPaths.push(args[i + 1]);
        i++;
      }
    }
  }

  // Create and start server
  const server = new CRAMCPServer({
    name: 'cra-mcp-server',
    version: '0.1.0',
    atlasPaths: atlasPaths.length > 0 ? atlasPaths : ['./atlases'],
    debug,
  });

  // Handle shutdown
  process.on('SIGINT', async () => {
    await server.stop();
    process.exit(0);
  });

  process.on('SIGTERM', async () => {
    await server.stop();
    process.exit(0);
  });

  // Start the server
  await server.start();
}

main().catch((error) => {
  console.error('Failed to start CRA MCP server:', error);
  process.exit(1);
});
