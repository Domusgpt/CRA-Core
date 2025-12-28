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
        version: '0.1.0',
        uptime_ms: Date.now() - this.startTime.getTime(),
      });
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
