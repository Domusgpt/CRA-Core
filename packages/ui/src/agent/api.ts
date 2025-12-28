/**
 * CRA Agent API
 *
 * Structured data output for agent consumption.
 * This provides optimized JSON responses that agents can ingest
 * without making extra tool calls or parsing HTML.
 */

/**
 * Agent dashboard data - everything needed in one response
 */
export interface AgentDashboardData {
  /** System overview */
  system: {
    name: string;
    version: string;
    status: 'healthy' | 'degraded' | 'unhealthy';
    uptime_ms: number;
  };

  /** Quick stats */
  stats: {
    atlases_loaded: number;
    resolutions_total: number;
    actions_executed: number;
    active_traces: number;
  };

  /** Available actions summary */
  actions: {
    available: string[];
    categories: Record<string, number>;
  };

  /** API endpoints for agent use */
  endpoints: {
    resolve: string;
    execute: string;
    trace: string;
    discover: string;
  };

  /** Suggested next actions */
  suggestions: string[];
}

/**
 * Agent terminal data - command context
 */
export interface AgentTerminalData {
  /** Available commands */
  commands: Array<{
    name: string;
    description: string;
    usage: string;
    example?: string;
  }>;

  /** API context */
  api: {
    base_url: string;
    ws_url: string;
  };

  /** Recent activity */
  recent: Array<{
    timestamp: string;
    type: string;
    summary: string;
  }>;
}

/**
 * Agent traces data - structured trace info
 */
export interface AgentTracesData {
  /** Total trace count */
  total: number;

  /** Recent traces with summary */
  traces: Array<{
    trace_id: string;
    started: string;
    event_count: number;
    duration_ms?: number;
    status: 'active' | 'completed' | 'failed';
    summary: string;
  }>;

  /** Streaming endpoint */
  stream_url: string;
}

/**
 * Single trace data for agents
 */
export interface AgentTraceData {
  trace_id: string;
  events: Array<{
    event_id: string;
    event_type: string;
    timestamp: string;
    severity: string;
    span_id: string;
    parent_span_id?: string;
    payload?: Record<string, unknown>;
  }>;
  summary: {
    total_events: number;
    duration_ms?: number;
    status: 'active' | 'completed' | 'failed';
    spans: number;
  };
}

/**
 * Full agent context - everything in one call
 */
export interface AgentFullContext {
  /** Timestamp of this context snapshot */
  generated_at: string;

  /** System info */
  system: {
    name: string;
    version: string;
    purpose: string;
    status: 'healthy' | 'degraded' | 'unhealthy';
  };

  /** How to use this system */
  usage: {
    resolve: {
      endpoint: string;
      method: 'POST';
      body: Record<string, unknown>;
      description: string;
    };
    execute: {
      endpoint: string;
      method: 'POST';
      body: Record<string, unknown>;
      description: string;
    };
    trace: {
      endpoint: string;
      method: 'GET' | 'WS';
      description: string;
    };
  };

  /** Current capabilities */
  capabilities: {
    atlases: string[];
    actions: string[];
    integrations: string[];
  };

  /** Runtime state */
  runtime: {
    uptime_ms: number;
    resolutions: number;
    executions: number;
    active_traces: number;
  };

  /** Recommended setup for agents.md */
  agents_md_config: string;
}

/**
 * Agent API - provides structured data for agent consumption
 */
export class AgentAPI {
  private readonly apiBaseUrl: string;

  constructor(apiBaseUrl: string = 'http://localhost:3000') {
    this.apiBaseUrl = apiBaseUrl;
  }

  /**
   * Get dashboard data optimized for agents
   */
  getDashboardData(): AgentDashboardData {
    return {
      system: {
        name: 'CRA - Context Registry Agents',
        version: '0.2.0',
        status: 'healthy',
        uptime_ms: 0, // Would be populated from actual server
      },
      stats: {
        atlases_loaded: 0,
        resolutions_total: 0,
        actions_executed: 0,
        active_traces: 0,
      },
      actions: {
        available: [],
        categories: {},
      },
      endpoints: {
        resolve: `${this.apiBaseUrl}/v1/resolve`,
        execute: `${this.apiBaseUrl}/v1/execute`,
        trace: `${this.apiBaseUrl}/v1/traces`,
        discover: `${this.apiBaseUrl}/v1/discover`,
      },
      suggestions: [
        'Call /v1/discover to get full system context',
        'Use /v1/resolve to check available actions',
        'Stream traces via WebSocket at /v1/trace',
      ],
    };
  }

  /**
   * Get terminal data for agents
   */
  getTerminalData(): AgentTerminalData {
    return {
      commands: [
        {
          name: 'resolve',
          description: 'Resolve context to get available actions',
          usage: 'POST /v1/resolve',
          example: '{"context": {"query": "...", "platform": "claude"}, "intent": "..."}',
        },
        {
          name: 'execute',
          description: 'Execute an action from resolution',
          usage: 'POST /v1/execute',
          example: '{"action_id": "...", "params": {...}}',
        },
        {
          name: 'discover',
          description: 'Get full API documentation and context',
          usage: 'GET /v1/discover',
        },
        {
          name: 'traces',
          description: 'List recent traces',
          usage: 'GET /v1/traces',
        },
        {
          name: 'stats',
          description: 'Get system statistics',
          usage: 'GET /v1/stats',
        },
      ],
      api: {
        base_url: this.apiBaseUrl,
        ws_url: this.apiBaseUrl.replace('http', 'ws') + '/v1/trace',
      },
      recent: [],
    };
  }

  /**
   * Get traces data for agents
   */
  getTracesData(): AgentTracesData {
    return {
      total: 0,
      traces: [],
      stream_url: this.apiBaseUrl.replace('http', 'ws') + '/v1/trace',
    };
  }

  /**
   * Get specific trace data for agents
   */
  getTraceData(traceId: string): AgentTraceData {
    return {
      trace_id: traceId,
      events: [],
      summary: {
        total_events: 0,
        status: 'active',
        spans: 0,
      },
    };
  }

  /**
   * Get full context for agents - everything in one call
   */
  getFullContext(): AgentFullContext {
    const wsUrl = this.apiBaseUrl.replace('http', 'ws');

    return {
      generated_at: new Date().toISOString(),
      system: {
        name: 'CRA - Context Registry Agents',
        version: '0.2.0',
        purpose:
          'Authority layer for agentic systems. Ask CRA what you can do before doing it. TRACE everything.',
        status: 'healthy',
      },
      usage: {
        resolve: {
          endpoint: `${this.apiBaseUrl}/v1/resolve`,
          method: 'POST',
          body: {
            context: {
              query: 'string - what you want to do',
              platform: 'string - your platform (claude, gpt, etc)',
              user: 'object - optional user context',
            },
            intent: 'string - describe your intent',
          },
          description:
            'Resolve your context to get available actions. Always call this before executing.',
        },
        execute: {
          endpoint: `${this.apiBaseUrl}/v1/execute`,
          method: 'POST',
          body: {
            action_id: 'string - from resolution response',
            params: 'object - action parameters',
            resolution_id: 'string - from resolution response',
          },
          description: 'Execute an action you received from resolution.',
        },
        trace: {
          endpoint: `${wsUrl}/v1/trace`,
          method: 'WS',
          description:
            'WebSocket stream of all TRACE events. Connect for real-time observability.',
        },
      },
      capabilities: {
        atlases: [],
        actions: [],
        integrations: ['opentelemetry', 'prometheus', 'mcp'],
      },
      runtime: {
        uptime_ms: 0,
        resolutions: 0,
        executions: 0,
        active_traces: 0,
      },
      agents_md_config: this.generateAgentsMdConfig(),
    };
  }

  /**
   * Generate agents.md configuration snippet
   */
  private generateAgentsMdConfig(): string {
    return `# CRA Integration

## Context Registry

When making decisions or taking actions:

1. **Resolve First**: Always call \`POST ${this.apiBaseUrl}/v1/resolve\` with your context
2. **Check Permissions**: Review the \`allowed_actions\` in the response
3. **Execute Safely**: Use the provided action_ids with \`POST ${this.apiBaseUrl}/v1/execute\`
4. **Trace Everything**: All operations are automatically traced

## Quick Start

\`\`\`javascript
// Step 1: Resolve your context
const resolution = await fetch('${this.apiBaseUrl}/v1/resolve', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'X-Agent-Id': 'your-agent-id'
  },
  body: JSON.stringify({
    context: { query: 'what I want to do', platform: 'claude' },
    intent: 'describe the intent'
  })
}).then(r => r.json());

// Step 2: Execute allowed action
const result = await fetch('${this.apiBaseUrl}/v1/execute', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    action_id: resolution.allowed_actions[0].action_id,
    resolution_id: resolution.resolution_id,
    params: { /* action params */ }
  })
}).then(r => r.json());
\`\`\`

## Headers

Include these headers for best observability:
- \`X-Agent-Id\`: Your unique agent identifier
- \`X-Session-Id\`: Current session/conversation ID
- \`X-Agent-Mode: true\`: Get structured JSON responses

## Resources

- Discovery: \`GET ${this.apiBaseUrl}/v1/discover\`
- Stats: \`GET ${this.apiBaseUrl}/v1/stats\`
- Traces: \`WS ${this.apiBaseUrl.replace('http', 'ws')}/v1/trace\`
`;
  }
}
