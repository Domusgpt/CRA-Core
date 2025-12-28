/**
 * CRA MCP Server Implementation
 *
 * Implements the Model Context Protocol server for CRA.
 */

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
  ListPromptsRequestSchema,
  GetPromptRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';
import { CRARuntime, createRequest } from '@cra/runtime';
import { MCPAdapter } from '@cra/adapters';
import type { ActionPermission, ContextBlock } from '@cra/protocol';

/**
 * MCP Server configuration
 */
export interface MCPServerConfig {
  /** Server name */
  name?: string;

  /** Server version */
  version?: string;

  /** Atlas paths to load */
  atlasPaths?: string[];

  /** Default agent ID */
  agentId?: string;

  /** Enable debug logging */
  debug?: boolean;
}

/**
 * CRA MCP Server
 *
 * Exposes CRA as an MCP server with:
 * - Tools: CARP actions from loaded atlases
 * - Resources: Context blocks from loaded atlases
 * - Prompts: Pre-configured resolution templates
 */
export class CRAMCPServer {
  private readonly server: Server;
  private readonly runtime: CRARuntime;
  private readonly adapter: MCPAdapter;
  private readonly config: Required<MCPServerConfig>;

  // Cached data from resolution
  private cachedActions: ActionPermission[] = [];
  private cachedContext: ContextBlock[] = [];
  private currentResolutionId: string | null = null;

  constructor(config: MCPServerConfig = {}) {
    this.config = {
      name: config.name ?? 'cra-mcp-server',
      version: config.version ?? '0.1.0',
      atlasPaths: config.atlasPaths ?? [],
      agentId: config.agentId ?? 'mcp-client',
      debug: config.debug ?? false,
    };

    // Initialize CRA runtime
    this.runtime = new CRARuntime({
      trace_to_file: false,
    });

    // Initialize MCP adapter
    this.adapter = new MCPAdapter({
      platform: 'mcp',
    });

    // Initialize MCP server
    this.server = new Server(
      {
        name: this.config.name,
        version: this.config.version,
      },
      {
        capabilities: {
          tools: {},
          resources: {},
          prompts: {},
        },
      }
    );

    this.setupHandlers();
  }

  /**
   * Set up MCP request handlers
   */
  private setupHandlers(): void {
    // List available tools
    this.server.setRequestHandler(ListToolsRequestSchema, async () => {
      const tools = this.adapter.toMCPTools(this.cachedActions);
      return {
        tools: tools.map(tool => ({
          name: tool.name,
          description: tool.description,
          inputSchema: tool.inputSchema as Record<string, unknown>,
        })),
      };
    });

    // Call a tool
    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;

      if (this.config.debug) {
        console.error(`[CRA-MCP] Tool call: ${name}`, args);
      }

      try {
        // Parse the tool call
        const action = this.adapter.parseMCPToolCall({
          method: 'tools/call',
          params: { name, arguments: args ?? {} },
        });

        // Check if action is allowed
        const allowed = this.cachedActions.find(
          a => a.action_type === action.action_type
        );

        if (!allowed) {
          return {
            content: [{ type: 'text' as const, text: JSON.stringify({ error: `Action ${action.action_type} is not permitted` }) }],
            isError: true,
          };
        }

        // Execute the action via CRA runtime
        if (this.currentResolutionId) {
          const result = await this.runtime.execute({
            carp_version: '1.0',
            request_id: `mcp-${Date.now()}`,
            timestamp: new Date().toISOString(),
            operation: 'execute',
            requester: {
              agent_id: this.config.agentId,
              session_id: this.runtime.getSessionId(),
            },
            action: {
              action_id: `act-${Date.now()}`,
              action_type: action.action_type,
              resolution_id: this.currentResolutionId,
              parameters: action.parameters,
            },
          });

          if ('error' in result) {
            return {
              content: [{ type: 'text' as const, text: JSON.stringify(result.error) }],
              isError: true,
            };
          }

          return {
            content: [{ type: 'text' as const, text: JSON.stringify(result.result) }],
          };
        }

        // No resolution - just return the action info
        return {
          content: [{ type: 'text' as const, text: JSON.stringify({
            action: action.action_type,
            parameters: action.parameters,
            note: 'Action received (no active resolution)',
          }) }],
        };
      } catch (error) {
        return {
          content: [{ type: 'text' as const, text: JSON.stringify({ error: String(error) }) }],
          isError: true,
        };
      }
    });

    // List available resources
    this.server.setRequestHandler(ListResourcesRequestSchema, async () => {
      const resources = this.adapter.toMCPResources(this.cachedContext);
      return {
        resources: resources.map(r => ({
          uri: r.uri,
          name: r.name,
          description: r.description,
          mimeType: r.mimeType,
        })),
      };
    });

    // Read a resource
    this.server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      const { uri } = request.params;

      const response = this.adapter.getResourceContent(uri, this.cachedContext);
      if (!response) {
        throw new Error(`Resource not found: ${uri}`);
      }

      return {
        contents: response.contents.map(c => ({
          uri: c.uri,
          mimeType: c.mimeType ?? 'text/plain',
          text: c.text,
        })),
      };
    });

    // List available prompts
    this.server.setRequestHandler(ListPromptsRequestSchema, async () => {
      return {
        prompts: [
          {
            name: 'resolve',
            description: 'Resolve context and permissions for a goal',
            arguments: [
              {
                name: 'goal',
                description: 'The goal or task to resolve context for',
                required: true,
              },
              {
                name: 'risk_tier',
                description: 'Risk tier (low, medium, high, critical)',
                required: false,
              },
            ],
          },
          {
            name: 'refresh',
            description: 'Refresh the current resolution',
          },
        ],
      };
    });

    // Get a prompt
    this.server.setRequestHandler(GetPromptRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;

      if (name === 'resolve') {
        const goal = args?.goal as string ?? 'general assistance';
        const riskTier = args?.risk_tier as string ?? 'medium';

        // Perform resolution
        await this.resolveContext(goal, riskTier);

        // Build system prompt
        const systemPrompt = this.adapter.toSystemPrompt(this.cachedContext);

        return {
          messages: [
            {
              role: 'user' as const,
              content: {
                type: 'text' as const,
                text: `Context has been resolved for: ${goal}\n\n${systemPrompt}`,
              },
            },
          ],
        };
      }

      if (name === 'refresh') {
        // Re-resolve with current settings
        const systemPrompt = this.adapter.toSystemPrompt(this.cachedContext);
        return {
          messages: [
            {
              role: 'user' as const,
              content: {
                type: 'text' as const,
                text: `Context refreshed.\n\n${systemPrompt}`,
              },
            },
          ],
        };
      }

      throw new Error(`Unknown prompt: ${name}`);
    });
  }

  /**
   * Resolve context for a goal
   */
  async resolveContext(goal: string, riskTier = 'medium'): Promise<void> {
    const request = createRequest(
      'resolve',
      {
        agent_id: this.config.agentId,
        session_id: this.runtime.getSessionId(),
      },
      {
        task: {
          goal,
          risk_tier: riskTier as 'low' | 'medium' | 'high' | 'critical',
        },
      }
    );

    const result = await this.runtime.resolve(request);

    if ('error' in result) {
      throw new Error(`Resolution failed: ${result.error.message}`);
    }

    // Cache the resolution results
    this.cachedActions = result.allowed_actions;
    this.cachedContext = result.context_blocks;
    this.currentResolutionId = result.resolution_id;

    if (this.config.debug) {
      console.error(`[CRA-MCP] Resolution complete: ${result.resolution_id}`);
      console.error(`[CRA-MCP] Actions: ${this.cachedActions.length}`);
      console.error(`[CRA-MCP] Context blocks: ${this.cachedContext.length}`);
    }
  }

  /**
   * Load atlases into the runtime
   */
  async loadAtlases(): Promise<void> {
    for (const atlasPath of this.config.atlasPaths) {
      try {
        await this.runtime.loadAtlas(atlasPath);
        if (this.config.debug) {
          console.error(`[CRA-MCP] Loaded atlas: ${atlasPath}`);
        }
      } catch (error) {
        console.error(`[CRA-MCP] Failed to load atlas ${atlasPath}: ${error}`);
      }
    }
  }

  /**
   * Start the MCP server using stdio transport
   */
  async start(): Promise<void> {
    // Load atlases first
    await this.loadAtlases();

    // Perform initial resolution
    await this.resolveContext('general assistance');

    // Connect via stdio
    const transport = new StdioServerTransport();
    await this.server.connect(transport);

    if (this.config.debug) {
      console.error(`[CRA-MCP] Server started: ${this.config.name}`);
    }
  }

  /**
   * Stop the server
   */
  async stop(): Promise<void> {
    await this.server.close();
    await this.runtime.shutdown();
  }

  /**
   * Get current cached actions
   */
  getActions(): ActionPermission[] {
    return this.cachedActions;
  }

  /**
   * Get current cached context
   */
  getContext(): ContextBlock[] {
    return this.cachedContext;
  }
}
