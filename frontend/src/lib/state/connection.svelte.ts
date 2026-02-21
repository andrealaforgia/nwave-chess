import { wsClient } from '../ws/client';

let connected = $state(false);
let reconnecting = $state(false);

export function initConnectionState(): void {
  wsClient.setStatusCallback(() => {
    connected = wsClient.connected;
    reconnecting = wsClient.reconnecting;
  });
}

export function getConnectionState() {
  return {
    get connected() {
      return connected;
    },
    get reconnecting() {
      return reconnecting;
    },
  };
}
