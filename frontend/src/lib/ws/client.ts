import type { ClientMessage, ServerMessage, ServerMessageType } from './protocol';

type MessageHandler = (msg: ServerMessage) => void;

const MIN_RECONNECT_DELAY = 1000;
const MAX_RECONNECT_DELAY = 30000;

class WebSocketClient {
  private ws: WebSocket | null = null;
  private handlers = new Map<ServerMessageType, MessageHandler[]>();
  private queue: ClientMessage[] = [];
  private reconnectDelay = MIN_RECONNECT_DELAY;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private intentionalClose = false;
  private _connected = false;
  private _reconnecting = false;
  private onStatusChange: (() => void) | null = null;

  get connected(): boolean {
    return this._connected;
  }

  get reconnecting(): boolean {
    return this._reconnecting;
  }

  setStatusCallback(cb: () => void): void {
    this.onStatusChange = cb;
  }

  connect(): void {
    if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) {
      return;
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const url = `${protocol}//${host}/ws`;

    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      this._connected = true;
      this._reconnecting = false;
      this.reconnectDelay = MIN_RECONNECT_DELAY;
      this.onStatusChange?.();
      this.flushQueue();
    };

    this.ws.onmessage = (event: MessageEvent) => {
      try {
        const msg = JSON.parse(event.data) as ServerMessage;
        this.dispatch(msg);
      } catch (e) {
        console.error('Failed to parse WebSocket message:', e);
      }
    };

    this.ws.onclose = () => {
      this._connected = false;
      this.onStatusChange?.();
      if (!this.intentionalClose) {
        this.scheduleReconnect();
      }
    };

    this.ws.onerror = () => {
      // onclose will fire after this, triggering reconnect
    };
  }

  disconnect(): void {
    this.intentionalClose = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.ws?.close();
    this.ws = null;
    this._connected = false;
    this._reconnecting = false;
    this.onStatusChange?.();
  }

  send(msg: ClientMessage): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    } else {
      this.queue.push(msg);
    }
  }

  onMessage(type: ServerMessageType, handler: MessageHandler): () => void {
    const handlers = this.handlers.get(type) ?? [];
    handlers.push(handler);
    this.handlers.set(type, handlers);

    return () => {
      const current = this.handlers.get(type);
      if (current) {
        this.handlers.set(
          type,
          current.filter((h) => h !== handler)
        );
      }
    };
  }

  private dispatch(msg: ServerMessage): void {
    const handlers = this.handlers.get(msg.type);
    if (handlers) {
      for (const handler of handlers) {
        handler(msg);
      }
    }
  }

  private flushQueue(): void {
    while (this.queue.length > 0) {
      const msg = this.queue.shift()!;
      this.send(msg);
    }
  }

  private scheduleReconnect(): void {
    this._reconnecting = true;
    this.onStatusChange?.();

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, this.reconnectDelay);

    this.reconnectDelay = Math.min(this.reconnectDelay * 2, MAX_RECONNECT_DELAY);
  }
}

export const wsClient = new WebSocketClient();
