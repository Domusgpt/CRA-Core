/**
 * CRA UI Server
 *
 * Serves the dual-mode web interface.
 */

import express, { Request, Response, Router } from 'express';
import { generateTerminalHTML } from './components/terminal.js';
import { generateTraceViewer } from './components/trace-viewer.js';
import { AgentAPI } from './agent/api.js';

/**
 * UI Server configuration
 */
export interface UIServerConfig {
  /** CRA API base URL */
  apiBaseUrl?: string;

  /** WebSocket URL for trace streaming */
  wsUrl?: string;

  /** Enable agent mode detection */
  enableAgentMode?: boolean;
}

/**
 * Create UI router for embedding in CRA server
 */
export function createUIRouter(config: UIServerConfig = {}): Router {
  const router = Router();
  const agentAPI = new AgentAPI(config.apiBaseUrl ?? 'http://localhost:3000');

  // Mode detection middleware
  router.use((req: Request, _res: Response, next) => {
    const isAgent =
      req.headers['x-agent-mode'] === 'true' ||
      req.query.format === 'agent' ||
      req.headers.accept === 'application/json';

    (req as any).isAgentMode = isAgent;
    next();
  });

  // Main dashboard
  router.get('/', (req: Request, res: Response) => {
    if ((req as any).isAgentMode) {
      res.json(agentAPI.getDashboardData());
      return;
    }

    res.type('html').send(generateDashboardHTML(config));
  });

  // Terminal interface
  router.get('/terminal', (req: Request, res: Response) => {
    if ((req as any).isAgentMode) {
      res.json(agentAPI.getTerminalData());
      return;
    }

    res.type('html').send(generateTerminalHTML(config));
  });

  // Trace viewer
  router.get('/traces', (req: Request, res: Response) => {
    if ((req as any).isAgentMode) {
      res.json(agentAPI.getTracesData());
      return;
    }

    res.type('html').send(generateTraceViewer(config));
  });

  // Trace viewer for specific trace
  router.get('/traces/:traceId', (req: Request, res: Response) => {
    const { traceId } = req.params;

    if ((req as any).isAgentMode) {
      res.json(agentAPI.getTraceData(traceId));
      return;
    }

    res.type('html').send(generateTraceViewer({ ...config, traceId }));
  });

  // Agent-specific endpoint - always returns structured data
  router.get('/agent', (_req: Request, res: Response) => {
    res.json(agentAPI.getFullContext());
  });

  return router;
}

/**
 * UI Server class for standalone operation
 */
export class UIServer {
  private readonly app: express.Application;
  private readonly serverConfig: UIServerConfig;

  constructor(config: UIServerConfig = {}) {
    this.serverConfig = config;
    this.app = express();
    this.app.use('/ui', createUIRouter(config));
  }

  getApp(): express.Application {
    return this.app;
  }

  getConfig(): UIServerConfig {
    return this.serverConfig;
  }

  async start(port: number = 3001): Promise<void> {
    return new Promise((resolve) => {
      this.app.listen(port, () => {
        console.log(`CRA UI running on http://localhost:${port}/ui`);
        resolve();
      });
    });
  }
}

/**
 * Generate main dashboard HTML
 */
function generateDashboardHTML(config: UIServerConfig): string {
  const apiBaseUrl = config.apiBaseUrl ?? 'http://localhost:3000';
  const wsUrl = config.wsUrl ?? 'ws://localhost:3000/v1/trace';

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>CRA - Context Registry Agents</title>
  <style>
    :root {
      --bg-dark: #0d1117;
      --bg-panel: #161b22;
      --border: #30363d;
      --text: #e6edf3;
      --text-muted: #8b949e;
      --accent: #58a6ff;
      --success: #3fb950;
      --warning: #d29922;
      --error: #f85149;
      --purple: #a371f7;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; }

    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: var(--bg-dark);
      color: var(--text);
      min-height: 100vh;
    }

    .header {
      background: var(--bg-panel);
      border-bottom: 1px solid var(--border);
      padding: 1rem 2rem;
      display: flex;
      align-items: center;
      justify-content: space-between;
    }

    .logo {
      font-size: 1.5rem;
      font-weight: 600;
      color: var(--accent);
    }

    .nav {
      display: flex;
      gap: 1rem;
    }

    .nav a {
      color: var(--text-muted);
      text-decoration: none;
      padding: 0.5rem 1rem;
      border-radius: 6px;
      transition: all 0.2s;
    }

    .nav a:hover, .nav a.active {
      color: var(--text);
      background: var(--bg-dark);
    }

    .container {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 1rem;
      padding: 1rem 2rem;
    }

    .panel {
      background: var(--bg-panel);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 1rem;
    }

    .panel-title {
      font-size: 1rem;
      font-weight: 600;
      margin-bottom: 1rem;
      color: var(--accent);
    }

    .stats-grid {
      display: grid;
      grid-template-columns: repeat(2, 1fr);
      gap: 1rem;
    }

    .stat {
      background: var(--bg-dark);
      padding: 1rem;
      border-radius: 6px;
    }

    .stat-value {
      font-size: 2rem;
      font-weight: 700;
      color: var(--accent);
    }

    .stat-label {
      color: var(--text-muted);
      font-size: 0.875rem;
    }

    .trace-stream {
      font-family: 'SF Mono', Monaco, Consolas, monospace;
      font-size: 0.8rem;
      max-height: 400px;
      overflow-y: auto;
      background: var(--bg-dark);
      border-radius: 6px;
      padding: 1rem;
    }

    .trace-event {
      padding: 0.25rem 0;
      border-bottom: 1px solid var(--border);
    }

    .trace-time {
      color: var(--text-muted);
    }

    .trace-type {
      color: var(--purple);
    }

    .quick-actions {
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }

    .btn {
      background: var(--accent);
      color: var(--bg-dark);
      border: none;
      padding: 0.75rem 1rem;
      border-radius: 6px;
      cursor: pointer;
      font-size: 0.875rem;
      font-weight: 500;
      text-decoration: none;
      text-align: center;
      transition: opacity 0.2s;
    }

    .btn:hover {
      opacity: 0.9;
    }

    .btn-secondary {
      background: var(--bg-dark);
      color: var(--text);
      border: 1px solid var(--border);
    }

    .full-width {
      grid-column: 1 / -1;
    }

    .mode-toggle {
      display: flex;
      gap: 0.5rem;
      align-items: center;
    }

    .mode-toggle span {
      color: var(--text-muted);
      font-size: 0.875rem;
    }
  </style>
</head>
<body>
  <header class="header">
    <div class="logo">CRA</div>
    <nav class="nav">
      <a href="/ui" class="active">Dashboard</a>
      <a href="/ui/terminal">Terminal</a>
      <a href="/ui/traces">Traces</a>
    </nav>
    <div class="mode-toggle">
      <span>Human Mode</span>
      <a href="/ui?format=agent" class="btn btn-secondary" style="padding: 0.25rem 0.5rem; font-size: 0.75rem;">Switch to Agent</a>
    </div>
  </header>

  <div class="container">
    <div class="panel">
      <h2 class="panel-title">System Status</h2>
      <div class="stats-grid" id="stats">
        <div class="stat">
          <div class="stat-value" id="atlases">-</div>
          <div class="stat-label">Atlases Loaded</div>
        </div>
        <div class="stat">
          <div class="stat-value" id="resolutions">-</div>
          <div class="stat-label">Resolutions</div>
        </div>
        <div class="stat">
          <div class="stat-value" id="actions">-</div>
          <div class="stat-label">Actions Executed</div>
        </div>
        <div class="stat">
          <div class="stat-value" id="uptime">-</div>
          <div class="stat-label">Uptime</div>
        </div>
      </div>
    </div>

    <div class="panel">
      <h2 class="panel-title">Quick Actions</h2>
      <div class="quick-actions">
        <a href="/ui/terminal" class="btn">Open Terminal</a>
        <a href="${apiBaseUrl}/v1/discover" target="_blank" class="btn btn-secondary">View Discovery API</a>
        <a href="${apiBaseUrl}/v1/discover?generate=agents-md" target="_blank" class="btn btn-secondary">Generate agents.md</a>
        <a href="/ui/traces" class="btn btn-secondary">View Traces</a>
      </div>
    </div>

    <div class="panel full-width">
      <h2 class="panel-title">Live Trace Stream</h2>
      <div class="trace-stream" id="trace-stream">
        <div class="trace-event">
          <span class="trace-time">Connecting to trace stream...</span>
        </div>
      </div>
    </div>
  </div>

  <script>
    const API_URL = '${apiBaseUrl}';
    const WS_URL = '${wsUrl}';

    // Fetch stats
    async function fetchStats() {
      try {
        const res = await fetch(API_URL + '/v1/stats');
        const data = await res.json();

        document.getElementById('atlases').textContent = data.runtime?.atlases_loaded ?? 0;
        document.getElementById('resolutions').textContent = data.runtime?.resolutions_total ?? 0;
        document.getElementById('actions').textContent = data.runtime?.actions_executed ?? 0;
        document.getElementById('uptime').textContent = formatUptime(data.uptime_ms);
      } catch (e) {
        console.error('Failed to fetch stats:', e);
      }
    }

    function formatUptime(ms) {
      if (!ms) return '-';
      const secs = Math.floor(ms / 1000);
      const mins = Math.floor(secs / 60);
      const hours = Math.floor(mins / 60);
      if (hours > 0) return hours + 'h ' + (mins % 60) + 'm';
      if (mins > 0) return mins + 'm ' + (secs % 60) + 's';
      return secs + 's';
    }

    // Connect to trace stream
    function connectTraceStream() {
      const stream = document.getElementById('trace-stream');

      try {
        const ws = new WebSocket(WS_URL);

        ws.onopen = () => {
          stream.innerHTML = '<div class="trace-event"><span class="trace-time">Connected to trace stream</span></div>';
        };

        ws.onmessage = (e) => {
          const event = JSON.parse(e.data);
          const time = new Date(event.timestamp).toISOString().slice(11, 23);
          const div = document.createElement('div');
          div.className = 'trace-event';
          div.innerHTML = '<span class="trace-time">' + time + '</span> <span class="trace-type">' + event.event_type + '</span>';
          stream.insertBefore(div, stream.firstChild);

          // Keep only last 50 events
          while (stream.children.length > 50) {
            stream.removeChild(stream.lastChild);
          }
        };

        ws.onerror = () => {
          stream.innerHTML = '<div class="trace-event"><span class="trace-time">WebSocket error - retrying...</span></div>';
          setTimeout(connectTraceStream, 3000);
        };

        ws.onclose = () => {
          setTimeout(connectTraceStream, 3000);
        };
      } catch (e) {
        stream.innerHTML = '<div class="trace-event"><span class="trace-time">WebSocket not available</span></div>';
      }
    }

    // Initialize
    fetchStats();
    setInterval(fetchStats, 5000);
    connectTraceStream();
  </script>
</body>
</html>`;
}
