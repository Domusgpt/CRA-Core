/**
 * CRA UI Package Tests
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
  UIServer,
  createUIRouter,
  generateTerminalHTML,
  generateTraceViewer,
  AgentAPI,
} from '../src/index.js';

describe('UIServer', () => {
  it('should create server with default config', () => {
    const server = new UIServer();
    expect(server).toBeDefined();
    expect(server.getApp()).toBeDefined();
  });

  it('should create server with custom config', () => {
    const server = new UIServer({
      apiBaseUrl: 'http://localhost:4000',
      wsUrl: 'ws://localhost:4000/trace',
      enableAgentMode: true,
    });
    expect(server).toBeDefined();
  });

  it('should return express app', () => {
    const server = new UIServer();
    const app = server.getApp();
    expect(app).toBeDefined();
    expect(typeof app.use).toBe('function');
  });
});

describe('createUIRouter', () => {
  it('should create router with default config', () => {
    const router = createUIRouter();
    expect(router).toBeDefined();
  });

  it('should create router with custom config', () => {
    const router = createUIRouter({
      apiBaseUrl: 'http://localhost:5000',
    });
    expect(router).toBeDefined();
  });
});

describe('generateTerminalHTML', () => {
  it('should generate HTML with default config', () => {
    const html = generateTerminalHTML({});
    expect(html).toContain('<!DOCTYPE html>');
    expect(html).toContain('CRA Terminal');
    expect(html).toContain('http://localhost:3000');
  });

  it('should include custom API URL', () => {
    const html = generateTerminalHTML({
      apiBaseUrl: 'http://custom:8080',
    });
    expect(html).toContain('http://custom:8080');
  });

  it('should include custom WebSocket URL', () => {
    const html = generateTerminalHTML({
      wsUrl: 'ws://custom:8080/trace',
    });
    expect(html).toContain('ws://custom:8080/trace');
  });

  it('should include terminal interface elements', () => {
    const html = generateTerminalHTML({});
    expect(html).toContain('terminal-input');
    expect(html).toContain('terminal-output');
    expect(html).toContain('cra>');
  });

  it('should include command handlers', () => {
    const html = generateTerminalHTML({});
    expect(html).toContain('commandHandlers');
    expect(html).toContain('help');
    expect(html).toContain('discover');
    expect(html).toContain('resolve');
    expect(html).toContain('execute');
  });
});

describe('generateTraceViewer', () => {
  it('should generate HTML with default config', () => {
    const html = generateTraceViewer({});
    expect(html).toContain('<!DOCTYPE html>');
    expect(html).toContain('CRA Trace Viewer');
    expect(html).toContain('http://localhost:3000');
  });

  it('should include trace ID in title when provided', () => {
    const html = generateTraceViewer({
      traceId: 'abc123',
    });
    expect(html).toContain('abc123');
  });

  it('should include trace viewer elements', () => {
    const html = generateTraceViewer({});
    expect(html).toContain('trace-list');
    expect(html).toContain('timeline');
    expect(html).toContain('json-view');
  });

  it('should include view toggle', () => {
    const html = generateTraceViewer({});
    expect(html).toContain('view-toggle');
    expect(html).toContain('Timeline');
    expect(html).toContain('JSON');
  });
});

describe('AgentAPI', () => {
  let api: AgentAPI;

  beforeEach(() => {
    api = new AgentAPI('http://localhost:3000');
  });

  describe('getDashboardData', () => {
    it('should return structured dashboard data', () => {
      const data = api.getDashboardData();

      expect(data.system).toBeDefined();
      expect(data.system.name).toBe('CRA - Context Registry Agents');
      expect(data.system.version).toBe('0.2.0');
      expect(data.system.status).toBe('healthy');

      expect(data.stats).toBeDefined();
      expect(typeof data.stats.atlases_loaded).toBe('number');
      expect(typeof data.stats.resolutions_total).toBe('number');

      expect(data.endpoints).toBeDefined();
      expect(data.endpoints.resolve).toContain('/v1/resolve');
      expect(data.endpoints.execute).toContain('/v1/execute');

      expect(data.suggestions).toBeInstanceOf(Array);
    });
  });

  describe('getTerminalData', () => {
    it('should return terminal command info', () => {
      const data = api.getTerminalData();

      expect(data.commands).toBeInstanceOf(Array);
      expect(data.commands.length).toBeGreaterThan(0);

      const resolveCmd = data.commands.find((c) => c.name === 'resolve');
      expect(resolveCmd).toBeDefined();
      expect(resolveCmd?.description).toBeDefined();
      expect(resolveCmd?.usage).toBeDefined();

      expect(data.api.base_url).toBe('http://localhost:3000');
      expect(data.api.ws_url).toContain('ws://');
    });
  });

  describe('getTracesData', () => {
    it('should return traces structure', () => {
      const data = api.getTracesData();

      expect(typeof data.total).toBe('number');
      expect(data.traces).toBeInstanceOf(Array);
      expect(data.stream_url).toContain('ws://');
    });
  });

  describe('getTraceData', () => {
    it('should return single trace structure', () => {
      const data = api.getTraceData('trace-123');

      expect(data.trace_id).toBe('trace-123');
      expect(data.events).toBeInstanceOf(Array);
      expect(data.summary).toBeDefined();
      expect(data.summary.status).toBe('active');
    });
  });

  describe('getFullContext', () => {
    it('should return complete context for agents', () => {
      const data = api.getFullContext();

      expect(data.generated_at).toBeDefined();
      expect(new Date(data.generated_at)).toBeInstanceOf(Date);

      expect(data.system.name).toBe('CRA - Context Registry Agents');
      expect(data.system.purpose).toContain('Authority layer');

      expect(data.usage.resolve.endpoint).toContain('/v1/resolve');
      expect(data.usage.resolve.method).toBe('POST');
      expect(data.usage.execute.endpoint).toContain('/v1/execute');
      expect(data.usage.trace.method).toBe('WS');

      expect(data.capabilities.integrations).toContain('opentelemetry');
      expect(data.capabilities.integrations).toContain('mcp');

      expect(data.agents_md_config).toContain('CRA Integration');
      expect(data.agents_md_config).toContain('/v1/resolve');
    });

    it('should include agents.md configuration', () => {
      const data = api.getFullContext();

      expect(data.agents_md_config).toContain('Quick Start');
      expect(data.agents_md_config).toContain('Headers');
      expect(data.agents_md_config).toContain('X-Agent-Id');
    });
  });

  describe('custom base URL', () => {
    it('should use custom base URL in endpoints', () => {
      const customApi = new AgentAPI('http://custom:8080');
      const data = customApi.getDashboardData();

      expect(data.endpoints.resolve).toBe('http://custom:8080/v1/resolve');
      expect(data.endpoints.execute).toBe('http://custom:8080/v1/execute');
    });

    it('should convert http to ws for WebSocket URLs', () => {
      const customApi = new AgentAPI('http://custom:8080');
      const data = customApi.getTerminalData();

      expect(data.api.ws_url).toBe('ws://custom:8080/v1/trace');
    });
  });
});

describe('Dual-mode behavior', () => {
  it('terminal HTML contains mode toggle', () => {
    const html = generateTerminalHTML({});
    // Terminal is human-focused but should have reference to API
    expect(html).toContain('Dashboard');
    expect(html).toContain('Terminal');
    expect(html).toContain('Traces');
  });

  it('agent API returns structured JSON-ready data', () => {
    const api = new AgentAPI();
    const dashboard = api.getDashboardData();

    // Should be JSON-serializable
    const serialized = JSON.stringify(dashboard);
    const parsed = JSON.parse(serialized);

    expect(parsed.system.name).toBe(dashboard.system.name);
    expect(parsed.endpoints.resolve).toBe(dashboard.endpoints.resolve);
  });
});
