// Message types
export type MessageRole = 'user' | 'assistant' | 'system' | 'error' | 'command' | 'tool-indicator';

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: Date;
}

// Gateway types
export interface GatewayMessage {
  id?: string;
  type?: 'event';
  event?: string;
  data?: unknown;
  result?: unknown;
  error?: {
    code: number;
    message: string;
  };
}

export interface RpcRequest {
  id: string;
  method: string;
  params?: Record<string, unknown>;
}

// Execution progress types
export interface ExecutionTask {
  id: string;
  parentId?: string;
  name: string;
  activeForm?: string;
  status: 'pending' | 'in_progress' | 'completed' | 'error';
  startTime?: number;
  endTime?: number;
  duration?: number;
  toolsCount?: number;
  tokensUsed?: number;
  children: ExecutionTask[];
}

export interface ExecutionEvent {
  execution_id: string;
  task_id?: string;
  parent_task_id?: string;
  name?: string;
  active_form?: string;
  status?: string;
  tools_count?: number;
  tokens_used?: number;
  duration_ms?: number;
  message?: string;
}

// API types
export interface ApiResponse<T> {
  data?: T;
  error?: string;
}

export interface AuthValidateResponse {
  valid: boolean;
  user?: {
    id: string;
    role: string;
  };
}

export interface AgentSettings {
  provider?: string;
  model?: string;
  context_size?: number;
  temperature?: number;
}

export interface Tool {
  name: string;
  description?: string;
  enabled: boolean;
  builtin: boolean;
}

export interface Skill {
  id: string;
  name: string;
  description?: string;
  version?: string;
  enabled: boolean;
}

export interface Session {
  id: string;
  channel_type: string;
  channel_id: string;
  created_at: string;
  updated_at: string;
  message_count?: number;
}

export interface Memory {
  id: string;
  content: string;
  importance?: number;
  created_at: string;
}

export interface Identity {
  id: string;
  name: string;
  description?: string;
  created_at: string;
}

export interface LogEntry {
  id: string;
  level: 'debug' | 'info' | 'warn' | 'error';
  message: string;
  timestamp: string;
  metadata?: Record<string, unknown>;
}

export interface Channel {
  id: string;
  type: 'telegram' | 'slack' | 'discord';
  name: string;
  enabled: boolean;
  config?: Record<string, unknown>;
}

// Slash command types
export interface SlashCommand {
  name: string;
  description: string;
  handler: () => void | Promise<void>;
}
