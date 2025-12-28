/**
 * CRA HTTP Server
 *
 * REST API server for Context Registry Agents.
 * Provides endpoints for CARP resolution, action execution, and trace streaming.
 */

import express, { Request, Response, NextFunction } from 'express';
import cors from 'cors';
import helmet from 'helmet';
import rateLimit from 'express-rate-limit';
import { WebSocketServer, WebSocket } from 'ws';
import { createServer, Server as HTTPServer } from 'http';
import { CRARuntime, createRequest } from '@cra/runtime';
import { getAdapter } from '@cra/adapters';
import type { CARPActionRequest, TRACEEvent } from '@cra/protocol';
import { generateId, getTimestamp, CARP_VERSION } from '@cra/protocol';

// =============================================================================
// Types
// =============================================================================

export interface ServerConfig {
  /** Server port */
  port?: number;
  /** Host to bind to */
  host?: string;
  /** Enable CORS */
  cors?: boolean;
  /** CORS origins */
  corsOrigins?: string[];
  /** Rate limit (requests per minute) */
  rateLimit?: number;
  /** API key for authentication (optional) */
  apiKey?: string;
  /** Enable WebSocket trace streaming */
  enableWebSocket?: boolean;
  /** Atlas paths to load */
  atlasPaths?: string[];
  /** Trace output directory */
  traceDir?: string;
}

export interface ServerStats {
  uptime_ms: number;
  requests_total: number;
  requests_success: number;
  requests_error: number;
  active_sessions: number;
  websocket_connections: number;
}

// =============================================================================
// API Key Authentication Middleware
// =============================================================================

function apiKeyAuth(apiKey?: string) {
  return (req: Request, res: Response, next: NextFunction) => {
    if (!apiKey) {
      return next(); // No API key required
    }

    const providedKey = req.headers['x-api-key'] || req.headers['authorization']?.replace('Bearer ', '');

    if (providedKey !== apiKey) {
      return res.status(401).json({
        error: {
          code: 'UNAUTHORIZED',
          message: 'Invalid or missing API key',
        },
      });
    }

    next();
  };
}

// =============================================================================
// CRA Server
// =============================================================================

export class CRAServer {
  private readonly app: express.Application;
  private readonly config: Required<ServerConfig>;
  private readonly runtime: CRARuntime;
  private httpServer?: HTTPServer;
  private wss?: WebSocketServer;
  private wsClients: Set<WebSocket> = new Set();
  private stats: ServerStats;
  private startTime: Date;

  constructor(config: ServerConfig = {}) {
    this.startTime = new Date();
    this.config = {
      port: config.port ?? 3000,
      host: config.host ?? '0.0.0.0',
      cors: config.cors ?? true,
      corsOrigins: config.corsOrigins ?? ['*'],
      rateLimit: config.rateLimit ?? 100,
      apiKey: config.apiKey ?? '',
      enableWebSocket: config.enableWebSocket ?? true,
      atlasPaths: config.atlasPaths ?? ['./atlases'],
      traceDir: config.traceDir ?? './traces',
    };

    this.stats = {
      uptime_ms: 0,
      requests_total: 0,
      requests_success: 0,
      requests_error: 0,
      active_sessions: 0,
      websocket_connections: 0,
    };

    // Initialize runtime
    this.runtime = new CRARuntime({
      trace_dir: this.config.traceDir,
      trace_to_file: true,
    });

    // Initialize Express app
    this.app = express();
    this.setupMiddleware();
    this.setupRoutes();
    this.setupTraceStreaming();
  }

  /**
   * Setup middleware
   */
  private setupMiddleware(): void {
    // Security headers
    this.app.use(helmet({
      contentSecurityPolicy: false, // Allow for API usage
    }));

    // CORS
    if (this.config.cors) {
      this.app.use(cors({
        origin: this.config.corsOrigins,
        methods: ['GET', 'POST', 'OPTIONS'],
        allowedHeaders: ['Content-Type', 'Authorization', 'X-API-Key'],
      }));
    }

    // JSON body parsing
    this.app.use(express.json({ limit: '10mb' }));

    // Rate limiting
    this.app.use(rateLimit({
      windowMs: 60 * 1000, // 1 minute
      max: this.config.rateLimit,
      message: {
        error: {
          code: 'RATE_LIMITED',
          message: `Too many requests. Limit: ${this.config.rateLimit}/minute`,
        },
      },
    }));

    // API key authentication
    this.app.use('/v1', apiKeyAuth(this.config.apiKey));

    // Request tracking
    this.app.use((req: Request, _res: Response, next: NextFunction) => {
      this.stats.requests_total++;
      (req as any).startTime = Date.now();
      next();
    });
  }

  /**
   * Setup API routes
   */
  private setupRoutes(): void {
    // Health check
    this.app.get('/health', (_req: Request, res: Response) => {
      res.json({
        status: 'healthy',
        version: '0.2.0',
        uptime_ms: Date.now() - this.startTime.getTime(),
      });
    });

    // Agent Discovery Endpoint - THE key endpoint for agents
    this.app.get('/v1/discover', (req: Request, res: Response) => {
      const generate = req.query.generate as string;
      const format = req.query.format as string ||
        (req.headers.accept?.includes('text/markdown') ? 'markdown' : 'json');

      const runtimeStats = this.runtime.getStats();

      const discovery = {
        system: {
          name: 'CRA - Context Registry Agents',
          version: '0.2.0',
          purpose: 'Authority layer for agentic systems with context governance and telemetry',
          one_liner: 'Ask CRA what you can do before doing it. TRACE everything.',
        },

        quick_start: {
          step_1: 'POST /v1/resolve with your goal to get permitted actions',
          step_2: 'Include returned context_blocks in your system prompt',
          step_3: 'Only use actions from allowed_actions list',
          step_4: 'POST /v1/execute when taking permitted actions',
          step_5: 'Connect to WS /v1/trace for real-time telemetry',
        },

        integration: {
          agents_md_snippet: `## CRA Integration

Before any action:
1. POST /v1/resolve with your goal
2. Parse allowed_actions - ONLY use these
3. Include context_blocks in your reasoning
4. TRACE events are authoritative, not LLM narration

Quick resolve:
\`\`\`
POST ${req.protocol}://${req.get('host')}/v1/resolve
{"goal": "your task description"}
\`\`\``,
          recommended_system_prompt: 'You operate under CRA governance. Before any action, resolve context via /v1/resolve. Only use tools from allowed_actions. All operations are traced.',
          update_frequency: 'Resolve once per task. Re-resolve if context changes or resolution expires.',
        },

        endpoints: {
          discover: {
            method: 'GET',
            path: '/v1/discover',
            purpose: 'Agent onboarding and system discovery (YOU ARE HERE)',
            query_params: {
              generate: 'agents-md | context | tools',
              format: 'json | markdown',
            },
          },
          resolve: {
            method: 'POST',
            path: '/v1/resolve',
            purpose: 'Get context and permitted actions for a goal',
            required_fields: ['goal'],
            optional_fields: ['risk_tier', 'context_hints', 'scope'],
            example_request: { goal: 'Create a GitHub issue for the bug', risk_tier: 'medium' },
            returns: 'CARPResolution with context_blocks, allowed_actions, policies_applied',
          },
          execute: {
            method: 'POST',
            path: '/v1/execute',
            purpose: 'Execute a permitted action',
            required_fields: ['action_type', 'resolution_id'],
            optional_fields: ['action_id', 'parameters'],
            prerequisite: 'Must have valid resolution_id from /v1/resolve',
          },
          stream: {
            method: 'WebSocket',
            path: '/v1/stream',
            purpose: 'Real-time streaming for resolution and actions',
            message_types: ['resolution.started', 'context.chunk', 'actions.chunk', 'resolution.complete'],
          },
          trace: {
            method: 'WebSocket',
            path: '/v1/trace',
            purpose: 'Real-time TRACE event streaming',
            message_types: ['All TRACEEvent types'],
          },
          batch: {
            method: 'POST',
            path: '/v1/batch',
            purpose: 'Batch multiple operations in one request',
            max_operations: 100,
          },
        },

        current_state: {
          loaded_atlases: runtimeStats.atlases_loaded,
          resolutions_total: runtimeStats.resolutions_total,
          actions_executed: runtimeStats.actions_executed,
          active_sessions: this.stats.active_sessions,
          uptime_ms: runtimeStats.uptime_ms,
        },

        for_agents: {
          output_formats: {
            json: 'Default - structured, minimal tokens',
            jsonl: 'Streaming - line-delimited JSON',
            markdown: 'Human-readable with structure (use Accept: text/markdown)',
          },
          headers: {
            'X-Agent-Id': 'Your agent identifier (for tracking)',
            'X-Session-Id': 'Session identifier (for continuity)',
            'X-Agent-Mode': 'Set to "true" for minimal response format',
          },
          response_envelope: {
            _meta: 'Request metadata (timing, cache status)',
            data: 'The actual response payload',
            _agent_hints: 'Suggestions for next actions',
          },
          best_practices: [
            'Cache resolution_id - valid for TTL seconds',
            'Use WebSocket for long-running operations',
            'Check allowed_actions before any tool use',
            'Include context_blocks in your reasoning',
          ],
        },

        urls: {
          base: `${req.protocol}://${req.get('host')}`,
          resolve: `${req.protocol}://${req.get('host')}/v1/resolve`,
          execute: `${req.protocol}://${req.get('host')}/v1/execute`,
          trace_ws: `ws://${req.get('host')}/v1/trace`,
          stream_ws: `ws://${req.get('host')}/v1/stream`,
          ui: `${req.protocol}://${req.get('host')}/ui`,
        },
      };

      // Generate specific outputs
      if (generate === 'agents-md') {
        const agentsMd = this.generateAgentsMd(req);
        if (format === 'markdown') {
          res.type('text/markdown').send(agentsMd);
        } else {
          res.json({ agents_md: agentsMd });
        }
        return;
      }

      if (generate === 'context') {
        res.json({
          system_context: discovery.integration.recommended_system_prompt,
          quick_reference: discovery.quick_start,
          endpoints: Object.entries(discovery.endpoints).map(([name, info]) => ({
            name,
            ...info,
          })),
        });
        return;
      }

      if (generate === 'tools') {
        res.json({
          tools: [
            {
              name: 'cra_resolve',
              description: 'Resolve context and permitted actions for a goal before taking action',
              parameters: {
                type: 'object',
                properties: {
                  goal: { type: 'string', description: 'The task or goal to resolve' },
                  risk_tier: { type: 'string', enum: ['low', 'medium', 'high', 'critical'] },
                },
                required: ['goal'],
              },
            },
            {
              name: 'cra_execute',
              description: 'Execute a permitted action after resolution',
              parameters: {
                type: 'object',
                properties: {
                  action_type: { type: 'string', description: 'Action type from allowed_actions' },
                  resolution_id: { type: 'string', description: 'Resolution ID from cra_resolve' },
                  parameters: { type: 'object', description: 'Action-specific parameters' },
                },
                required: ['action_type', 'resolution_id'],
              },
            },
          ],
        });
        return;
      }

      res.json(discovery);
    });

    // Statistics
    this.app.get('/v1/stats', (_req: Request, res: Response) => {
      res.json({
        ...this.stats,
        uptime_ms: Date.now() - this.startTime.getTime(),
        websocket_connections: this.wsClients.size,
        runtime: this.runtime.getStats(),
      });
    });

    // CARP Resolve endpoint
    this.app.post('/v1/resolve', async (req: Request, res: Response) => {
      try {
        const { goal, risk_tier, context_hints, scope } = req.body;

        if (!goal) {
          this.stats.requests_error++;
          return res.status(400).json({
            error: {
              code: 'INVALID_REQUEST',
              message: 'Missing required field: goal',
            },
          });
        }

        // Create CARP request
        const request = createRequest('resolve', {
          agent_id: req.headers['x-agent-id'] as string || 'api-client',
          session_id: req.headers['x-session-id'] as string || this.runtime.getSessionId(),
        }, {
          task: {
            goal,
            risk_tier: risk_tier || 'medium',
            context_hints,
          },
          scope,
        });

        // Resolve
        const result = await this.runtime.resolve(request);

        if ('error' in result) {
          this.stats.requests_error++;
          return res.status(400).json(result);
        }

        this.stats.requests_success++;
        res.json(result);
      } catch (error) {
        this.stats.requests_error++;
        res.status(500).json({
          error: {
            code: 'INTERNAL_ERROR',
            message: String(error),
          },
        });
      }
    });

    // CARP Execute endpoint
    this.app.post('/v1/execute', async (req: Request, res: Response) => {
      try {
        const { resolution_id, action_id, action_type, parameters } = req.body;

        if (!resolution_id || !action_type) {
          this.stats.requests_error++;
          return res.status(400).json({
            error: {
              code: 'INVALID_REQUEST',
              message: 'Missing required fields: resolution_id, action_type',
            },
          });
        }

        // Create action request
        const actionRequest: CARPActionRequest = {
          carp_version: CARP_VERSION,
          request_id: generateId(),
          timestamp: getTimestamp(),
          operation: 'execute',
          requester: {
            agent_id: req.headers['x-agent-id'] as string || 'api-client',
            session_id: req.headers['x-session-id'] as string || this.runtime.getSessionId(),
          },
          action: {
            action_id: action_id || generateId(),
            action_type,
            resolution_id,
            parameters: parameters || {},
          },
        };

        // Execute
        const result = await this.runtime.execute(actionRequest);

        if ('error' in result) {
          this.stats.requests_error++;
          return res.status(400).json(result);
        }

        this.stats.requests_success++;
        res.json(result);
      } catch (error) {
        this.stats.requests_error++;
        res.status(500).json({
          error: {
            code: 'INTERNAL_ERROR',
            message: String(error),
          },
        });
      }
    });

    // Adapter configuration endpoint
    this.app.post('/v1/adapt/:platform', async (req: Request, res: Response) => {
      try {
        const { platform } = req.params;
        const { resolution } = req.body;

        if (!resolution) {
          this.stats.requests_error++;
          return res.status(400).json({
            error: {
              code: 'INVALID_REQUEST',
              message: 'Missing required field: resolution',
            },
          });
        }

        const adapter = getAdapter(platform as any);
        const config = adapter.getConfiguration(resolution);

        this.stats.requests_success++;
        res.json({
          platform,
          system_prompt: config.systemPrompt,
          tools: config.tools,
        });
      } catch (error) {
        this.stats.requests_error++;
        res.status(400).json({
          error: {
            code: 'ADAPTER_ERROR',
            message: String(error),
          },
        });
      }
    });

    // List loaded atlases
    this.app.get('/v1/atlases', (_req: Request, res: Response) => {
      // Note: This would need to be exposed by runtime
      res.json({
        atlases: [],
        message: 'Atlas listing not yet implemented',
      });
    });

    // Load an atlas
    this.app.post('/v1/atlases', async (req: Request, res: Response) => {
      try {
        const { path } = req.body;

        if (!path) {
          return res.status(400).json({
            error: {
              code: 'INVALID_REQUEST',
              message: 'Missing required field: path',
            },
          });
        }

        const atlas = await this.runtime.loadAtlas(path);
        res.json({
          loaded: true,
          atlas_ref: atlas.ref,
        });
      } catch (error) {
        res.status(400).json({
          error: {
            code: 'ATLAS_LOAD_ERROR',
            message: String(error),
          },
        });
      }
    });
  }

  /**
   * Setup WebSocket trace streaming
   */
  private setupTraceStreaming(): void {
    if (!this.config.enableWebSocket) return;

    // Stream trace events to WebSocket clients
    this.runtime.getTrace().on('event', (event: TRACEEvent) => {
      const message = JSON.stringify(event);
      for (const client of this.wsClients) {
        if (client.readyState === WebSocket.OPEN) {
          client.send(message);
        }
      }
    });
  }

  /**
   * Start the server
   */
  async start(): Promise<void> {
    // Load atlases
    for (const atlasPath of this.config.atlasPaths) {
      try {
        await this.runtime.loadAtlas(atlasPath);
        console.log(`Loaded atlas from ${atlasPath}`);
      } catch (error) {
        console.warn(`Failed to load atlas from ${atlasPath}: ${error}`);
      }
    }

    // Create HTTP server
    this.httpServer = createServer(this.app);

    // Setup WebSocket server
    if (this.config.enableWebSocket) {
      this.wss = new WebSocketServer({
        server: this.httpServer,
        path: '/v1/trace',
      });

      this.wss.on('connection', (ws: WebSocket) => {
        this.wsClients.add(ws);
        this.stats.websocket_connections = this.wsClients.size;

        ws.on('close', () => {
          this.wsClients.delete(ws);
          this.stats.websocket_connections = this.wsClients.size;
        });

        ws.on('error', () => {
          this.wsClients.delete(ws);
        });

        // Send welcome message
        ws.send(JSON.stringify({
          type: 'connected',
          trace_id: this.runtime.getTrace().getTraceId(),
          session_id: this.runtime.getSessionId(),
        }));
      });
    }

    // Start listening
    return new Promise((resolve) => {
      this.httpServer!.listen(this.config.port, this.config.host, () => {
        console.log(`CRA Server listening on http://${this.config.host}:${this.config.port}`);
        if (this.config.enableWebSocket) {
          console.log(`WebSocket trace streaming: ws://${this.config.host}:${this.config.port}/v1/trace`);
        }
        resolve();
      });
    });
  }

  /**
   * Stop the server
   */
  async stop(): Promise<void> {
    // Close WebSocket connections
    for (const client of this.wsClients) {
      client.close();
    }
    this.wsClients.clear();

    // Close WebSocket server
    if (this.wss) {
      this.wss.close();
    }

    // Close HTTP server
    if (this.httpServer) {
      await new Promise<void>((resolve) => {
        this.httpServer!.close(() => resolve());
      });
    }

    // Shutdown runtime
    await this.runtime.shutdown();
  }

  /**
   * Generate agents.md content for auto-configuration
   */
  private generateAgentsMd(req: Request): string {
    const baseUrl = `${req.protocol}://${req.get('host')}`;
    const runtimeStats = this.runtime.getStats();

    return `# agents.md — CRA Contract

> Auto-generated by CRA at ${new Date().toISOString()}
> Source: ${baseUrl}/v1/discover?generate=agents-md

## Overview

This project uses **CRA (Context Registry Agents)** for governance and telemetry.
All AI agent operations must go through CRA for context resolution and action permission.

## The Golden Rule

**Never guess what tools to use. Always ask CRA first.**

\`\`\`
Before action → POST /v1/resolve → Get allowed_actions → Only use those
\`\`\`

## Integration

### Step 1: Resolve Context
Before any task, resolve what you're allowed to do:

\`\`\`bash
curl -X POST ${baseUrl}/v1/resolve \\
  -H "Content-Type: application/json" \\
  -d '{"goal": "your task description", "risk_tier": "medium"}'
\`\`\`

### Step 2: Parse Response
The response contains:
- \`context_blocks\`: Domain knowledge to include in your reasoning
- \`allowed_actions\`: ONLY these tools are permitted
- \`resolution_id\`: Required for execution
- \`cache.ttl_seconds\`: How long this resolution is valid

### Step 3: Execute Actions
When ready to act, execute through CRA:

\`\`\`bash
curl -X POST ${baseUrl}/v1/execute \\
  -H "Content-Type: application/json" \\
  -d '{
    "action_type": "api.github.create_issue",
    "resolution_id": "res_xxx",
    "parameters": {"title": "...", "body": "..."}
  }'
\`\`\`

## Current State

- **Loaded Atlases**: ${runtimeStats.atlases_loaded}
- **Resolutions Processed**: ${runtimeStats.resolutions_total}
- **Actions Executed**: ${runtimeStats.actions_executed}
- **Server**: ${baseUrl}

## Risk Tiers

| Tier | Description | Approval |
|------|-------------|----------|
| low | Read-only, no side effects | Auto-approved |
| medium | Modifications with undo | Auto-approved, logged |
| high | Destructive, limited undo | May require approval |
| critical | Irreversible | Always requires approval |

## Real-Time Telemetry

Connect to WebSocket for live trace events:
\`\`\`javascript
const ws = new WebSocket('${baseUrl.replace('http', 'ws')}/v1/trace');
ws.onmessage = (e) => console.log(JSON.parse(e.data));
\`\`\`

## Important Notes

1. **TRACE is authoritative** - The telemetry stream is the source of truth, not LLM narration
2. **Resolution expires** - Check \`cache.ttl_seconds\` and re-resolve when needed
3. **Context matters** - Always include \`context_blocks\` in your reasoning
4. **Actions are typed** - Use exact \`action_type\` strings from \`allowed_actions\`

## Quick Reference

| Endpoint | Method | Purpose |
|----------|--------|---------|
| /v1/discover | GET | System discovery (start here) |
| /v1/resolve | POST | Get context and permissions |
| /v1/execute | POST | Execute permitted action |
| /v1/trace | WS | Real-time telemetry |
| /v1/stream | WS | Streaming resolution |

---
*CRA v0.2.0 - Context Registry Agents*
`;
  }

  /**
   * Get Express app (for testing)
   */
  getApp(): express.Application {
    return this.app;
  }

  /**
   * Get runtime (for testing)
   */
  getRuntime(): CRARuntime {
    return this.runtime;
  }
}
