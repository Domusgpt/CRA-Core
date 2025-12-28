/**
 * CRA UI Package
 *
 * Dual-mode web interface:
 * - Human Mode: Rich terminal-style interface with visualizations
 * - Agent Mode: Structured JSON responses optimized for AI consumption
 */

export { UIServer, createUIRouter, type UIServerConfig } from './server.js';
export { generateTerminalHTML } from './components/terminal.js';
export { generateTraceViewer, type TraceViewerConfig } from './components/trace-viewer.js';
export {
  AgentAPI,
  type AgentDashboardData,
  type AgentTerminalData,
  type AgentTracesData,
  type AgentTraceData,
  type AgentFullContext,
} from './agent/api.js';
