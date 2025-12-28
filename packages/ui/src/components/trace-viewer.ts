/**
 * CRA Trace Viewer Component
 *
 * Visual trace exploration interface.
 */

import type { UIServerConfig } from '../server.js';

export interface TraceViewerConfig extends UIServerConfig {
  /** Specific trace ID to display */
  traceId?: string;
}

/**
 * Generate trace viewer HTML interface
 */
export function generateTraceViewer(config: TraceViewerConfig): string {
  const apiBaseUrl = config.apiBaseUrl ?? 'http://localhost:3000';
  const wsUrl = config.wsUrl ?? 'ws://localhost:3000/v1/trace';
  const traceId = config.traceId ?? '';

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>CRA Trace Viewer${traceId ? ' - ' + traceId : ''}</title>
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
      --cyan: #56d4dd;
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

    .container {
      display: flex;
      height: calc(100vh - 53px);
    }

    .sidebar {
      width: 320px;
      background: var(--bg-panel);
      border-right: 1px solid var(--border);
      display: flex;
      flex-direction: column;
    }

    .sidebar-header {
      padding: 1rem;
      border-bottom: 1px solid var(--border);
    }

    .search-input {
      width: 100%;
      padding: 0.5rem 0.75rem;
      background: var(--bg-dark);
      border: 1px solid var(--border);
      border-radius: 6px;
      color: var(--text);
      font-size: 0.875rem;
      outline: none;
    }

    .search-input:focus {
      border-color: var(--accent);
    }

    .trace-list {
      flex: 1;
      overflow-y: auto;
    }

    .trace-item {
      padding: 0.75rem 1rem;
      border-bottom: 1px solid var(--border);
      cursor: pointer;
      transition: background 0.2s;
    }

    .trace-item:hover {
      background: var(--bg-dark);
    }

    .trace-item.active {
      background: var(--bg-dark);
      border-left: 3px solid var(--accent);
    }

    .trace-id {
      font-family: 'SF Mono', Monaco, monospace;
      font-size: 0.75rem;
      color: var(--accent);
      margin-bottom: 0.25rem;
    }

    .trace-meta {
      display: flex;
      justify-content: space-between;
      font-size: 0.75rem;
      color: var(--text-muted);
    }

    .trace-events {
      color: var(--purple);
    }

    .main {
      flex: 1;
      display: flex;
      flex-direction: column;
      overflow: hidden;
    }

    .main-header {
      padding: 1rem;
      border-bottom: 1px solid var(--border);
      display: flex;
      align-items: center;
      justify-content: space-between;
    }

    .trace-title {
      font-size: 1.125rem;
      font-weight: 600;
    }

    .view-toggle {
      display: flex;
      gap: 0.25rem;
      background: var(--bg-dark);
      padding: 0.25rem;
      border-radius: 6px;
    }

    .view-btn {
      padding: 0.375rem 0.75rem;
      background: transparent;
      border: none;
      color: var(--text-muted);
      font-size: 0.875rem;
      cursor: pointer;
      border-radius: 4px;
      transition: all 0.2s;
    }

    .view-btn.active {
      background: var(--accent);
      color: var(--bg-dark);
    }

    .timeline {
      flex: 1;
      overflow: auto;
      padding: 1rem;
    }

    .event {
      display: flex;
      gap: 1rem;
      margin-bottom: 1rem;
      position: relative;
    }

    .event::before {
      content: '';
      position: absolute;
      left: 11px;
      top: 24px;
      bottom: -1rem;
      width: 2px;
      background: var(--border);
    }

    .event:last-child::before {
      display: none;
    }

    .event-dot {
      width: 24px;
      height: 24px;
      border-radius: 50%;
      background: var(--bg-panel);
      border: 2px solid var(--accent);
      flex-shrink: 0;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 0.75rem;
      z-index: 1;
    }

    .event-dot.started {
      border-color: var(--success);
      color: var(--success);
    }

    .event-dot.completed {
      border-color: var(--accent);
      color: var(--accent);
    }

    .event-dot.failed {
      border-color: var(--error);
      color: var(--error);
    }

    .event-dot.info {
      border-color: var(--purple);
      color: var(--purple);
    }

    .event-content {
      flex: 1;
      background: var(--bg-panel);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 0.75rem 1rem;
    }

    .event-header {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      margin-bottom: 0.5rem;
    }

    .event-type {
      font-weight: 600;
      color: var(--text);
    }

    .event-time {
      font-size: 0.75rem;
      color: var(--text-muted);
      font-family: 'SF Mono', Monaco, monospace;
    }

    .event-details {
      font-size: 0.875rem;
      color: var(--text-muted);
    }

    .event-payload {
      margin-top: 0.5rem;
      padding: 0.5rem;
      background: var(--bg-dark);
      border-radius: 4px;
      font-family: 'SF Mono', Monaco, monospace;
      font-size: 0.75rem;
      overflow-x: auto;
    }

    .json-view {
      padding: 1rem;
      overflow: auto;
    }

    .json-content {
      font-family: 'SF Mono', Monaco, monospace;
      font-size: 0.875rem;
      white-space: pre-wrap;
      background: var(--bg-panel);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 1rem;
    }

    .empty-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: 100%;
      color: var(--text-muted);
      text-align: center;
      padding: 2rem;
    }

    .empty-state h3 {
      margin-bottom: 0.5rem;
      color: var(--text);
    }

    .live-indicator {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      font-size: 0.875rem;
      color: var(--success);
    }

    .live-dot {
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

    .severity-info { color: var(--accent); }
    .severity-warn { color: var(--warning); }
    .severity-error { color: var(--error); }
    .severity-debug { color: var(--text-muted); }
  </style>
</head>
<body>
  <header class="header">
    <div class="header-left">
      <div class="logo">CRA</div>
      <nav class="nav">
        <a href="/ui">Dashboard</a>
        <a href="/ui/terminal">Terminal</a>
        <a href="/ui/traces" class="active">Traces</a>
      </nav>
    </div>
    <div class="live-indicator">
      <span class="live-dot"></span>
      <span>Live</span>
    </div>
  </header>

  <div class="container">
    <aside class="sidebar">
      <div class="sidebar-header">
        <input type="text" class="search-input" id="search" placeholder="Search traces...">
      </div>
      <div class="trace-list" id="trace-list">
        <div class="trace-item">
          <div class="trace-id">Loading traces...</div>
        </div>
      </div>
    </aside>

    <main class="main">
      <div class="main-header">
        <h1 class="trace-title" id="trace-title">Select a trace</h1>
        <div class="view-toggle">
          <button class="view-btn active" data-view="timeline">Timeline</button>
          <button class="view-btn" data-view="json">JSON</button>
        </div>
      </div>

      <div class="timeline" id="timeline">
        <div class="empty-state">
          <h3>No trace selected</h3>
          <p>Select a trace from the sidebar to view its events</p>
        </div>
      </div>

      <div class="json-view" id="json-view" style="display: none;">
        <pre class="json-content" id="json-content"></pre>
      </div>
    </main>
  </div>

  <script>
    const API_URL = '${apiBaseUrl}';
    const WS_URL = '${wsUrl}';
    const INITIAL_TRACE_ID = '${traceId}';

    let currentView = 'timeline';
    let currentTrace = null;
    let traces = [];

    const traceList = document.getElementById('trace-list');
    const traceTitle = document.getElementById('trace-title');
    const timeline = document.getElementById('timeline');
    const jsonView = document.getElementById('json-view');
    const jsonContent = document.getElementById('json-content');
    const searchInput = document.getElementById('search');

    // View toggle
    document.querySelectorAll('.view-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        document.querySelector('.view-btn.active').classList.remove('active');
        btn.classList.add('active');
        currentView = btn.dataset.view;
        updateView();
      });
    });

    function updateView() {
      if (currentView === 'timeline') {
        timeline.style.display = 'block';
        jsonView.style.display = 'none';
      } else {
        timeline.style.display = 'none';
        jsonView.style.display = 'block';
        if (currentTrace) {
          jsonContent.textContent = JSON.stringify(currentTrace, null, 2);
        }
      }
    }

    // Fetch traces
    async function fetchTraces() {
      try {
        const res = await fetch(API_URL + '/v1/traces');
        traces = await res.json();
        renderTraceList();

        if (INITIAL_TRACE_ID) {
          loadTrace(INITIAL_TRACE_ID);
        }
      } catch (e) {
        traceList.innerHTML = '<div class="trace-item"><div class="trace-id">Failed to load traces</div></div>';
      }
    }

    function renderTraceList(filter = '') {
      const filtered = filter
        ? traces.filter(t => t.trace_id.includes(filter) || (t.events && t.events.some(e => e.event_type.includes(filter))))
        : traces;

      if (filtered.length === 0) {
        traceList.innerHTML = '<div class="trace-item"><div class="trace-id">No traces found</div></div>';
        return;
      }

      traceList.innerHTML = filtered.map(t => \`
        <div class="trace-item\${currentTrace && currentTrace.trace_id === t.trace_id ? ' active' : ''}" data-id="\${t.trace_id}">
          <div class="trace-id">\${t.trace_id.substring(0, 16)}...</div>
          <div class="trace-meta">
            <span class="trace-events">\${t.events?.length || 0} events</span>
            <span>\${formatTime(t.timestamp || t.events?.[0]?.timestamp)}</span>
          </div>
        </div>
      \`).join('');

      // Add click handlers
      traceList.querySelectorAll('.trace-item').forEach(item => {
        item.addEventListener('click', () => loadTrace(item.dataset.id));
      });
    }

    async function loadTrace(traceId) {
      try {
        const res = await fetch(API_URL + '/v1/traces/' + traceId);
        currentTrace = await res.json();
        renderTrace();

        // Update URL
        history.pushState({}, '', '/ui/traces/' + traceId);

        // Update active state in list
        traceList.querySelectorAll('.trace-item').forEach(item => {
          item.classList.toggle('active', item.dataset.id === traceId);
        });
      } catch (e) {
        console.error('Failed to load trace:', e);
      }
    }

    function renderTrace() {
      if (!currentTrace) return;

      traceTitle.textContent = 'Trace: ' + currentTrace.trace_id.substring(0, 16) + '...';

      const events = currentTrace.events || [currentTrace];

      timeline.innerHTML = events.map(event => \`
        <div class="event">
          <div class="event-dot \${getEventClass(event)}">
            \${getEventIcon(event)}
          </div>
          <div class="event-content">
            <div class="event-header">
              <span class="event-type">\${event.event_type}</span>
              <span class="event-time">\${formatTime(event.timestamp)}</span>
            </div>
            <div class="event-details">
              <span class="severity-\${event.severity}">\${event.severity}</span>
              &middot; Span: \${event.span_id?.substring(0, 8) || 'N/A'}
            </div>
            \${event.payload ? \`
              <div class="event-payload">\${JSON.stringify(event.payload, null, 2)}</div>
            \` : ''}
          </div>
        </div>
      \`).join('');

      updateView();
    }

    function getEventClass(event) {
      if (event.event_type.includes('started') || event.event_type.includes('begin')) return 'started';
      if (event.event_type.includes('completed') || event.event_type.includes('end')) return 'completed';
      if (event.event_type.includes('failed') || event.event_type.includes('error')) return 'failed';
      return 'info';
    }

    function getEventIcon(event) {
      if (event.event_type.includes('started')) return '▶';
      if (event.event_type.includes('completed')) return '✓';
      if (event.event_type.includes('failed')) return '✕';
      return '•';
    }

    function formatTime(timestamp) {
      if (!timestamp) return 'N/A';
      return new Date(timestamp).toLocaleTimeString();
    }

    // Search
    searchInput.addEventListener('input', (e) => {
      renderTraceList(e.target.value);
    });

    // WebSocket for live updates
    let ws = null;
    function connectWS() {
      try {
        ws = new WebSocket(WS_URL);
        ws.onmessage = (e) => {
          const event = JSON.parse(e.data);
          // Add to current trace if matching
          if (currentTrace && event.trace_id === currentTrace.trace_id) {
            if (!currentTrace.events) currentTrace.events = [];
            currentTrace.events.push(event);
            renderTrace();
          }
          // Refresh trace list
          fetchTraces();
        };
        ws.onerror = () => setTimeout(connectWS, 5000);
        ws.onclose = () => setTimeout(connectWS, 5000);
      } catch (e) {
        // WebSocket not available
      }
    }

    // Initialize
    fetchTraces();
    connectWS();
  </script>
</body>
</html>`;
}
