/**
 * CRA Server Package
 *
 * HTTP/WebSocket server for Context Registry Agents.
 */

export { CRAServer, ServerConfig, ServerStats } from './server.js';

// CLI entry point when run directly
if (import.meta.url === `file://${process.argv[1]}`) {
  const { CRAServer } = await import('./server.js');

  const server = new CRAServer({
    port: parseInt(process.env.CRA_PORT || '3000'),
    host: process.env.CRA_HOST || '0.0.0.0',
    apiKey: process.env.CRA_API_KEY,
    atlasPaths: process.env.CRA_ATLAS_PATHS?.split(',') || ['./atlases'],
    traceDir: process.env.CRA_TRACE_DIR || './traces',
    rateLimit: parseInt(process.env.CRA_RATE_LIMIT || '100'),
  });

  // Graceful shutdown
  process.on('SIGINT', async () => {
    console.log('\nShutting down...');
    await server.stop();
    process.exit(0);
  });

  process.on('SIGTERM', async () => {
    await server.stop();
    process.exit(0);
  });

  // Start server
  await server.start();

  console.log('\nEndpoints:');
  console.log('  POST /v1/resolve     - Resolve context and permissions');
  console.log('  POST /v1/execute     - Execute a permitted action');
  console.log('  POST /v1/adapt/:platform - Get platform-specific configuration');
  console.log('  POST /v1/atlases     - Load an atlas');
  console.log('  GET  /v1/stats       - Server statistics');
  console.log('  GET  /health         - Health check');
  console.log('  WS   /v1/trace       - WebSocket trace stream');
}
