<script lang="ts">
  import { onMount } from 'svelte';
  import ConnectionStatus from './lib/components/ConnectionStatus.svelte';
  import ColorSelectionScreen from './lib/components/ColorSelectionScreen.svelte';
  import GameScreen from './lib/components/GameScreen.svelte';
  import SelfPlayScreen from './lib/components/SelfPlayScreen.svelte';
  import { wsClient } from './lib/ws/client';
  import { initConnectionState } from './lib/state/connection.svelte';
  import { initGameState, getGameState } from './lib/state/game.svelte';
  import { initSearchState } from './lib/state/search.svelte';
  import { initLearningState } from './lib/state/learning.svelte';

  const game = getGameState();

  onMount(() => {
    initConnectionState();
    initGameState();
    initSearchState();
    initLearningState();
    wsClient.connect();

    return () => {
      wsClient.disconnect();
    };
  });
</script>

<div class="app">
  <header class="top-bar">
    <span class="app-title">nwave-chess</span>
    <ConnectionStatus />
  </header>
  <main class="content">
    {#if game.status === 'selecting_color' || game.status === 'self_play_config'}
      <ColorSelectionScreen />
    {:else if game.status === 'self_play_running'}
      <SelfPlayScreen />
    {:else}
      <GameScreen />
    {/if}
  </main>
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }

  .top-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 20px;
    background-color: #1a1a1a;
    border-bottom: 1px solid #333;
  }

  .app-title {
    font-size: 1rem;
    font-weight: 600;
    color: #888;
    letter-spacing: 1px;
  }

  .content {
    flex: 1;
    padding: 20px;
  }
</style>
