/**
 * Base Platform Adapter
 *
 * Abstract interface for translating CRA artifacts to platform-specific formats.
 */

import type { ActionPermission, ContextBlock, CARPResolution } from '@cra/protocol';
import type { Platform, AdapterConfig, ActionDefinition } from '@cra/atlas';

// =============================================================================
// Types
// =============================================================================

export interface PlatformTool {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
}

export interface PlatformToolCall {
  name: string;
  arguments: Record<string, unknown>;
  id?: string;
}

export interface TranslatedAction {
  action_type: string;
  parameters: Record<string, unknown>;
}

export interface AdapterOptions {
  platform: Platform;
  config?: AdapterConfig;
}

// =============================================================================
// Base Adapter
// =============================================================================

export abstract class BasePlatformAdapter {
  readonly platform: Platform;
  protected config?: AdapterConfig;

  constructor(options: AdapterOptions) {
    this.platform = options.platform;
    this.config = options.config;
  }

  /**
   * Convert CARP action permissions to platform tool definitions
   */
  abstract toToolDefinitions(actions: ActionPermission[]): PlatformTool[];

  /**
   * Convert context blocks to a system prompt
   */
  abstract toSystemPrompt(
    contextBlocks: ContextBlock[],
    options?: { prefix?: string; suffix?: string }
  ): string;

  /**
   * Translate a platform tool call to a CARP action
   */
  abstract fromToolCall(call: PlatformToolCall): TranslatedAction;

  /**
   * Get the full platform configuration for a resolution
   */
  getConfiguration(resolution: CARPResolution): {
    systemPrompt: string;
    tools: PlatformTool[];
  } {
    return {
      systemPrompt: this.toSystemPrompt(resolution.context_blocks),
      tools: this.toToolDefinitions(resolution.allowed_actions),
    };
  }
}

// =============================================================================
// Adapter Registry
// =============================================================================

const adapterRegistry = new Map<Platform, new (options: AdapterOptions) => BasePlatformAdapter>();

export function registerAdapter(
  platform: Platform,
  adapterClass: new (options: AdapterOptions) => BasePlatformAdapter
): void {
  adapterRegistry.set(platform, adapterClass);
}

export function getAdapter(platform: Platform, config?: AdapterConfig): BasePlatformAdapter {
  const AdapterClass = adapterRegistry.get(platform);
  if (!AdapterClass) {
    throw new Error(`No adapter registered for platform: ${platform}`);
  }
  return new AdapterClass({ platform, config });
}

export function getSupportedPlatforms(): Platform[] {
  return Array.from(adapterRegistry.keys());
}
