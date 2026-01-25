import type { GatewayMessage, RpcRequest } from '@/types';

type EventCallback = (data: unknown) => void;

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (reason: Error) => void;
  timeout: ReturnType<typeof setTimeout>;
}

export class GatewayClient {
  private url: string;
  private ws: WebSocket | null = null;
  private pendingRequests: Map<string, PendingRequest> = new Map();
  private eventListeners: Map<string, Set<EventCallback>> = new Map();
  private wildcardListeners: Set<EventCallback> = new Set();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private intentionalDisconnect = false;
  private connectionPromise: Promise<void> | null = null;
  private connectionResolve: (() => void) | null = null;

  constructor(url?: string) {
    if (url) {
      this.url = url;
    } else {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      // In development (Vite dev server on port 3001), use the /ws proxy path
      // In production (served from backend on port 8080), connect directly to gateway port
      const isDev = window.location.port === '3001';
      if (isDev) {
        // Use Vite proxy
        this.url = `${protocol}//${window.location.host}/ws`;
      } else {
        // Direct connection to gateway (production or when served from backend)
        const gatewayPort = '8081';
        this.url = `${protocol}//${window.location.hostname}:${gatewayPort}`;
      }
    }
  }

  connect(): Promise<void> {
    if (this.ws?.readyState === WebSocket.OPEN) {
      return Promise.resolve();
    }

    if (this.connectionPromise) {
      return this.connectionPromise;
    }

    this.connectionPromise = new Promise((resolve, reject) => {
      this.connectionResolve = resolve;

      try {
        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
          console.log('[Gateway] Connected to', this.url);
          this.reconnectAttempts = 0;
          this.reconnectDelay = 1000;
          this.emitEvent('connected', {});
          if (this.connectionResolve) {
            this.connectionResolve();
            this.connectionResolve = null;
          }
        };

        this.ws.onmessage = (event) => {
          this.handleMessage(event.data);
        };

        this.ws.onclose = () => {
          console.log('[Gateway] Connection closed');
          this.emitEvent('disconnected', {});
          this.connectionPromise = null;

          if (!this.intentionalDisconnect && this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            console.log(`[Gateway] Reconnecting in ${this.reconnectDelay}ms (attempt ${this.reconnectAttempts})`);
            setTimeout(() => this.connect(), this.reconnectDelay);
            this.reconnectDelay = Math.min(this.reconnectDelay * 2, 30000);
          }
        };

        this.ws.onerror = (error) => {
          console.error('[Gateway] WebSocket error:', error);
          this.emitEvent('error', { error });
          reject(error);
        };
      } catch (error) {
        this.connectionPromise = null;
        reject(error);
      }
    });

    return this.connectionPromise;
  }

  private handleMessage(data: string): void {
    try {
      const message: GatewayMessage = JSON.parse(data);

      // Handle server events
      if (message.type === 'event' && message.event) {
        this.emitEvent(message.event, message.data);
        return;
      }

      // Handle RPC responses
      if (message.id) {
        const pending = this.pendingRequests.get(message.id);
        if (pending) {
          clearTimeout(pending.timeout);
          this.pendingRequests.delete(message.id);

          if (message.error) {
            pending.reject(new Error(message.error.message || 'RPC error'));
          } else {
            pending.resolve(message.result);
          }
        }
      }
    } catch (error) {
      console.error('[Gateway] Failed to parse message:', error);
    }
  }

  async call<T = unknown>(method: string, params?: Record<string, unknown>): Promise<T> {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error('WebSocket not connected');
    }

    const id = crypto.randomUUID();
    const request: RpcRequest = { id, method, params };

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error('RPC call timed out'));
      }, 30000);

      this.pendingRequests.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
        timeout
      });

      this.ws!.send(JSON.stringify(request));
    });
  }

  on(event: string, callback: EventCallback): void {
    if (event === '*') {
      this.wildcardListeners.add(callback);
    } else {
      if (!this.eventListeners.has(event)) {
        this.eventListeners.set(event, new Set());
      }
      this.eventListeners.get(event)!.add(callback);
    }
  }

  off(event: string, callback: EventCallback): void {
    if (event === '*') {
      this.wildcardListeners.delete(callback);
    } else {
      this.eventListeners.get(event)?.delete(callback);
    }
  }

  private emitEvent(event: string, data: unknown): void {
    // Notify specific listeners
    const listeners = this.eventListeners.get(event);
    if (listeners) {
      listeners.forEach((callback) => {
        try {
          callback(data);
        } catch (error) {
          console.error(`[Gateway] Event handler error for ${event}:`, error);
        }
      });
    }

    // Notify wildcard listeners
    this.wildcardListeners.forEach((callback) => {
      try {
        callback({ event, data });
      } catch (error) {
        console.error('[Gateway] Wildcard handler error:', error);
      }
    });
  }

  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  disconnect(): void {
    this.intentionalDisconnect = true;
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.pendingRequests.forEach((pending) => {
      clearTimeout(pending.timeout);
      pending.reject(new Error('Connection closed'));
    });
    this.pendingRequests.clear();
    this.connectionPromise = null;
  }
}

// Singleton instance
let gatewayInstance: GatewayClient | null = null;

export function getGateway(): GatewayClient {
  if (!gatewayInstance) {
    gatewayInstance = new GatewayClient();
  }
  return gatewayInstance;
}

export function resetGateway(): void {
  if (gatewayInstance) {
    gatewayInstance.disconnect();
    gatewayInstance = null;
  }
}
