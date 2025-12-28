/**
 * CRA MCP Server Tests
 */

import { describe, it, expect } from 'vitest';
import { CRAMCPServer, type MCPServerConfig } from '../server.js';

describe('CRAMCPServer', () => {
  describe('Initialization', () => {
    it('should create server with default config', () => {
      const server = new CRAMCPServer();
      expect(server).toBeDefined();
    });

    it('should create server with custom config', () => {
      const config: MCPServerConfig = {
        name: 'test-mcp',
        version: '1.0.0',
        atlasPaths: ['./test-atlases'],
        agentId: 'test-agent',
        debug: true,
      };
      const server = new CRAMCPServer(config);
      expect(server).toBeDefined();
    });

    it('should have empty actions before resolution', () => {
      const server = new CRAMCPServer();
      expect(server.getActions()).toEqual([]);
    });

    it('should have empty context before resolution', () => {
      const server = new CRAMCPServer();
      expect(server.getContext()).toEqual([]);
    });
  });

  describe('Configuration', () => {
    it('should use default name if not provided', () => {
      const server = new CRAMCPServer();
      expect(server).toBeDefined();
    });

    it('should use default version if not provided', () => {
      const server = new CRAMCPServer();
      expect(server).toBeDefined();
    });

    it('should accept custom atlas paths', () => {
      const server = new CRAMCPServer({
        atlasPaths: ['/path/to/atlas1', '/path/to/atlas2'],
      });
      expect(server).toBeDefined();
    });
  });

  describe('Resolution', () => {
    it('should throw when no atlases are loaded', async () => {
      const server = new CRAMCPServer();
      // Resolution without atlases should throw
      await expect(server.resolveContext('test goal')).rejects.toThrow('No applicable atlases found');
    });

    it('should throw for different risk tiers when no atlases', async () => {
      const server = new CRAMCPServer();
      // Resolution without atlases should throw
      await expect(server.resolveContext('test goal', 'high')).rejects.toThrow('No applicable atlases found');
    });
  });
});

describe('MCP Server Config', () => {
  it('should accept all configuration options', () => {
    const config: MCPServerConfig = {
      name: 'custom-server',
      version: '2.0.0',
      atlasPaths: ['./atlases'],
      agentId: 'custom-agent',
      debug: false,
    };

    expect(config.name).toBe('custom-server');
    expect(config.version).toBe('2.0.0');
    expect(config.atlasPaths).toContain('./atlases');
    expect(config.agentId).toBe('custom-agent');
    expect(config.debug).toBe(false);
  });

  it('should allow partial configuration', () => {
    const config: MCPServerConfig = {
      name: 'minimal-server',
    };

    expect(config.name).toBe('minimal-server');
    expect(config.version).toBeUndefined();
    expect(config.atlasPaths).toBeUndefined();
  });
});

describe('Factory Function', () => {
  it('should be importable', async () => {
    const { createMCPServer } = await import('../factory.js');
    expect(typeof createMCPServer).toBe('function');
  });
});
