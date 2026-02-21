<script lang="ts">
  import { getConnectionState } from '../state/connection.svelte';

  const conn = getConnectionState();
</script>

<div class="connection-status">
  {#if conn.connected}
    <span class="dot green"></span>
    <span class="label">Connected</span>
  {:else if conn.reconnecting}
    <span class="dot yellow"></span>
    <span class="label">Reconnecting...</span>
  {:else}
    <span class="dot red"></span>
    <span class="label">Disconnected</span>
  {/if}
</div>

<style>
  .connection-status {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
    color: #aaa;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
    flex-shrink: 0;
  }

  .dot.green {
    background-color: #4caf50;
    box-shadow: 0 0 4px #4caf50;
  }

  .dot.yellow {
    background-color: #ff9800;
    box-shadow: 0 0 4px #ff9800;
    animation: pulse 1s infinite;
  }

  .dot.red {
    background-color: #f44336;
    box-shadow: 0 0 4px #f44336;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }

  .label {
    white-space: nowrap;
  }
</style>
