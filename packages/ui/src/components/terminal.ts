/**
 * CRA Terminal Component
 *
 * Interactive terminal interface for human users.
 */

import type { UIServerConfig } from '../server.js';

/**
 * Generate terminal HTML interface
 */
export function generateTerminalHTML(config: UIServerConfig): string {
  const apiBaseUrl = config.apiBaseUrl ?? 'http://localhost:3000';
  const wsUrl = config.wsUrl ?? 'ws://localhost:3000/v1/trace';

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>CRA Terminal</title>
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
      font-family: 'SF Mono', Monaco, Consolas, 'Liberation Mono', monospace;
      background: var(--bg-dark);
      color: var(--text);
      min-height: 100vh;
      display: flex;
      flex-direction: column;
    }

    .header {
      background: var(--bg-panel);
      border-bottom: 1px solid var(--border);
      padding: 0.75rem 1rem;
      display: flex;
      align-items: center;
      justify-content: space-between;
    }

    .header-left {
      display: flex;
      align-items: center;
      gap: 1rem;
    }

    .logo {
      font-size: 1.25rem;
      font-weight: 600;
      color: var(--accent);
    }

    .nav {
      display: flex;
      gap: 0.5rem;
    }

    .nav a {
      color: var(--text-muted);
      text-decoration: none;
      padding: 0.25rem 0.75rem;
      border-radius: 4px;
      font-size: 0.875rem;
    }

    .nav a:hover, .nav a.active {
      color: var(--text);
      background: var(--bg-dark);
    }

    .terminal-container {
      flex: 1;
      display: flex;
      flex-direction: column;
      padding: 1rem;
      gap: 1rem;
    }

    .terminal {
      flex: 1;
      background: var(--bg-panel);
      border: 1px solid var(--border);
      border-radius: 8px;
      display: flex;
      flex-direction: column;
      overflow: hidden;
    }

    .terminal-header {
      background: var(--bg-dark);
      padding: 0.5rem 1rem;
      border-bottom: 1px solid var(--border);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .terminal-dot {
      width: 12px;
      height: 12px;
      border-radius: 50%;
    }

    .dot-red { background: var(--error); }
    .dot-yellow { background: var(--warning); }
    .dot-green { background: var(--success); }

    .terminal-title {
      flex: 1;
      text-align: center;
      color: var(--text-muted);
      font-size: 0.875rem;
    }

    .terminal-output {
      flex: 1;
      padding: 1rem;
      overflow-y: auto;
      font-size: 0.875rem;
      line-height: 1.5;
    }

    .output-line {
      white-space: pre-wrap;
      word-break: break-all;
    }

    .output-line.command {
      color: var(--accent);
    }

    .output-line.success {
      color: var(--success);
    }

    .output-line.error {
      color: var(--error);
    }

    .output-line.info {
      color: var(--text-muted);
    }

    .output-line.json {
      color: var(--purple);
    }

    .terminal-input-container {
      display: flex;
      align-items: center;
      padding: 0.75rem 1rem;
      border-top: 1px solid var(--border);
      background: var(--bg-dark);
    }

    .prompt {
      color: var(--success);
      margin-right: 0.5rem;
    }

    .terminal-input {
      flex: 1;
      background: transparent;
      border: none;
      color: var(--text);
      font-family: inherit;
      font-size: 0.875rem;
      outline: none;
    }

    .sidebar {
      width: 280px;
      background: var(--bg-panel);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 1rem;
      display: flex;
      flex-direction: column;
      gap: 1rem;
    }

    .sidebar-section {
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }

    .sidebar-title {
      font-size: 0.75rem;
      font-weight: 600;
      text-transform: uppercase;
      color: var(--text-muted);
      letter-spacing: 0.05em;
    }

    .command-list {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    .command-item {
      display: flex;
      justify-content: space-between;
      padding: 0.375rem 0.5rem;
      background: var(--bg-dark);
      border-radius: 4px;
      font-size: 0.8rem;
      cursor: pointer;
      transition: background 0.2s;
    }

    .command-item:hover {
      background: var(--border);
    }

    .command-name {
      color: var(--accent);
    }

    .command-desc {
      color: var(--text-muted);
    }

    .main-content {
      display: flex;
      gap: 1rem;
      flex: 1;
    }

    .status-indicator {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      font-size: 0.875rem;
    }

    .status-dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--success);
      animation: pulse 2s infinite;
    }

    @keyframes pulse {
      0%, 100% { opacity: 1; }
      50% { opacity: 0.5; }
    }
  </style>
</head>
<body>
  <header class="header">
    <div class="header-left">
      <div class="logo">CRA</div>
      <nav class="nav">
        <a href="/ui">Dashboard</a>
        <a href="/ui/terminal" class="active">Terminal</a>
        <a href="/ui/traces">Traces</a>
      </nav>
    </div>
    <div class="status-indicator">
      <span class="status-dot"></span>
      <span>Connected</span>
    </div>
  </header>

  <div class="terminal-container">
    <div class="main-content">
      <div class="terminal">
        <div class="terminal-header">
          <span class="terminal-dot dot-red"></span>
          <span class="terminal-dot dot-yellow"></span>
          <span class="terminal-dot dot-green"></span>
          <span class="terminal-title">CRA Interactive Terminal</span>
        </div>
        <div class="terminal-output" id="output">
          <div class="output-line info">Welcome to CRA Terminal</div>
          <div class="output-line info">Type 'help' for available commands</div>
          <div class="output-line info">---</div>
        </div>
        <div class="terminal-input-container">
          <span class="prompt">cra&gt;</span>
          <input type="text" class="terminal-input" id="input" placeholder="Enter command..." autofocus>
        </div>
      </div>

      <div class="sidebar">
        <div class="sidebar-section">
          <div class="sidebar-title">Quick Commands</div>
          <div class="command-list" id="commands">
            <div class="command-item" data-cmd="help">
              <span class="command-name">help</span>
              <span class="command-desc">Show commands</span>
            </div>
            <div class="command-item" data-cmd="discover">
              <span class="command-name">discover</span>
              <span class="command-desc">API info</span>
            </div>
            <div class="command-item" data-cmd="stats">
              <span class="command-name">stats</span>
              <span class="command-desc">System stats</span>
            </div>
            <div class="command-item" data-cmd="atlases">
              <span class="command-name">atlases</span>
              <span class="command-desc">List atlases</span>
            </div>
            <div class="command-item" data-cmd="resolve">
              <span class="command-name">resolve</span>
              <span class="command-desc">Resolve context</span>
            </div>
            <div class="command-item" data-cmd="traces">
              <span class="command-name">traces</span>
              <span class="command-desc">View traces</span>
            </div>
            <div class="command-item" data-cmd="clear">
              <span class="command-name">clear</span>
              <span class="command-desc">Clear output</span>
            </div>
          </div>
        </div>

        <div class="sidebar-section">
          <div class="sidebar-title">Connection</div>
          <div style="font-size: 0.8rem; color: var(--text-muted);">
            API: ${apiBaseUrl}<br>
            WS: ${wsUrl}
          </div>
        </div>
      </div>
    </div>
  </div>

  <script>
    const API_URL = '${apiBaseUrl}';
    const WS_URL = '${wsUrl}';
    const output = document.getElementById('output');
    const input = document.getElementById('input');
    const commands = document.getElementById('commands');
    let history = [];
    let historyIndex = -1;

    // Command handlers
    const commandHandlers = {
      help: () => {
        return \`Available commands:
  help      - Show this help message
  discover  - Show API discovery information
  stats     - Show system statistics
  atlases   - List loaded atlases
  resolve   - Resolve context (usage: resolve <query>)
  execute   - Execute action (usage: execute <action_id>)
  traces    - View recent traces
  trace     - View specific trace (usage: trace <trace_id>)
  clear     - Clear terminal output
  ws        - Toggle WebSocket stream\`;
      },

      clear: () => {
        output.innerHTML = '';
        return null;
      },

      discover: async () => {
        const res = await fetch(API_URL + '/v1/discover');
        return await res.json();
      },

      stats: async () => {
        const res = await fetch(API_URL + '/v1/stats');
        return await res.json();
      },

      atlases: async () => {
        const res = await fetch(API_URL + '/v1/atlases');
        return await res.json();
      },

      traces: async () => {
        const res = await fetch(API_URL + '/v1/traces');
        return await res.json();
      },

      resolve: async (args) => {
        if (!args) {
          return { error: 'Usage: resolve <query>' };
        }
        const res = await fetch(API_URL + '/v1/resolve', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            context: { query: args },
            intent: args
          })
        });
        return await res.json();
      },

      execute: async (args) => {
        if (!args) {
          return { error: 'Usage: execute <action_id> [params_json]' };
        }
        const [actionId, ...rest] = args.split(' ');
        const params = rest.length > 0 ? JSON.parse(rest.join(' ')) : {};
        const res = await fetch(API_URL + '/v1/execute', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            action_id: actionId,
            params
          })
        });
        return await res.json();
      },

      trace: async (args) => {
        if (!args) {
          return { error: 'Usage: trace <trace_id>' };
        }
        const res = await fetch(API_URL + '/v1/traces/' + args);
        return await res.json();
      }
    };

    // Add output line
    function addLine(text, className = '') {
      const line = document.createElement('div');
      line.className = 'output-line ' + className;
      if (typeof text === 'object') {
        line.className += ' json';
        line.textContent = JSON.stringify(text, null, 2);
      } else {
        line.textContent = text;
      }
      output.appendChild(line);
      output.scrollTop = output.scrollHeight;
    }

    // Execute command
    async function executeCommand(cmd) {
      const [command, ...args] = cmd.trim().split(' ');
      const argStr = args.join(' ');

      addLine('cra> ' + cmd, 'command');

      if (!command) return;

      history.push(cmd);
      historyIndex = history.length;

      const handler = commandHandlers[command.toLowerCase()];
      if (!handler) {
        addLine('Unknown command: ' + command + '. Type "help" for available commands.', 'error');
        return;
      }

      try {
        const result = await handler(argStr);
        if (result !== null && result !== undefined) {
          if (result.error) {
            addLine(result.error, 'error');
          } else {
            addLine(result, 'success');
          }
        }
      } catch (e) {
        addLine('Error: ' + e.message, 'error');
      }
    }

    // Input handling
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        const cmd = input.value;
        input.value = '';
        executeCommand(cmd);
      } else if (e.key === 'ArrowUp') {
        if (historyIndex > 0) {
          historyIndex--;
          input.value = history[historyIndex];
        }
        e.preventDefault();
      } else if (e.key === 'ArrowDown') {
        if (historyIndex < history.length - 1) {
          historyIndex++;
          input.value = history[historyIndex];
        } else {
          historyIndex = history.length;
          input.value = '';
        }
        e.preventDefault();
      }
    });

    // Quick command clicks
    commands.addEventListener('click', (e) => {
      const item = e.target.closest('.command-item');
      if (item) {
        const cmd = item.dataset.cmd;
        input.value = cmd;
        input.focus();
      }
    });

    // Focus input on page click
    document.body.addEventListener('click', (e) => {
      if (!e.target.closest('.sidebar')) {
        input.focus();
      }
    });

    // WebSocket for live updates
    let ws = null;
    function connectWS() {
      try {
        ws = new WebSocket(WS_URL);
        ws.onmessage = (e) => {
          const event = JSON.parse(e.data);
          addLine('[TRACE] ' + event.event_type, 'info');
        };
        ws.onerror = () => setTimeout(connectWS, 5000);
        ws.onclose = () => setTimeout(connectWS, 5000);
      } catch (e) {
        // WebSocket not available
      }
    }
    connectWS();
  </script>
</body>
</html>`;
}
