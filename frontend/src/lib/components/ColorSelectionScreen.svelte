<script lang="ts">
  import {
    selectColor,
    enterSelfPlayConfig,
    exitSelfPlayConfig,
    startSelfPlay,
    getGameState,
  } from '../state/game.svelte';

  const game = getGameState();

  let numGames = $state(10);

  function handleStartSelfPlay() {
    if (numGames > 0 && numGames <= 1000) {
      startSelfPlay(numGames);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      handleStartSelfPlay();
    }
  }
</script>

<div class="color-selection">
  <h1>nwave-chess</h1>
  <p class="subtitle">Self-learning chess engine</p>

  {#if game.status === 'self_play_config'}
    <div class="self-play-config">
      <p class="config-label">Number of games:</p>
      <input
        type="number"
        class="game-count-input"
        bind:value={numGames}
        min="1"
        max="1000"
        onkeydown={handleKeydown}
      />
      <div class="config-buttons">
        <button class="action-btn start-btn" onclick={handleStartSelfPlay}>
          Start Self-Play
        </button>
        <button class="action-btn back-btn" onclick={exitSelfPlayConfig}>
          Back
        </button>
      </div>
    </div>
  {:else}
    <div class="buttons">
      <button class="color-btn white-btn" onclick={() => selectColor('white')}>
        <span class="piece-icon">&#9812;</span>
        <span>Play as White</span>
      </button>
      <button class="color-btn black-btn" onclick={() => selectColor('black')}>
        <span class="piece-icon">&#9818;</span>
        <span>Play as Black</span>
      </button>
    </div>
    <button class="self-play-btn" onclick={enterSelfPlayConfig}>
      <span class="self-play-icon">&#8634;</span>
      <span>Self-Play Mode</span>
    </button>
  {/if}
</div>

<style>
  .color-selection {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 80vh;
    gap: 16px;
  }

  h1 {
    font-size: 2.5rem;
    margin: 0;
    color: #e0e0e0;
    letter-spacing: 2px;
  }

  .subtitle {
    color: #888;
    margin: 0 0 32px 0;
    font-size: 1rem;
  }

  .buttons {
    display: flex;
    gap: 24px;
  }

  .color-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 32px 48px;
    border-radius: 12px;
    border: 2px solid #444;
    font-size: 1.1rem;
    cursor: pointer;
    transition:
      border-color 0.2s,
      background-color 0.2s,
      transform 0.15s;
    background-color: #2a2a2a;
    color: #e0e0e0;
  }

  .color-btn:hover {
    transform: translateY(-2px);
    border-color: #7b61ff;
    background-color: #333;
  }

  .piece-icon {
    font-size: 3rem;
    line-height: 1;
  }

  .white-btn .piece-icon {
    color: #f0d9b5;
  }

  .black-btn .piece-icon {
    color: #b58863;
  }

  .self-play-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 24px;
    padding: 12px 32px;
    border-radius: 8px;
    border: 1px solid #555;
    font-size: 1rem;
    cursor: pointer;
    background-color: #1e1e1e;
    color: #aaa;
    transition:
      border-color 0.2s,
      color 0.2s;
  }

  .self-play-btn:hover {
    border-color: #7b61ff;
    color: #e0e0e0;
  }

  .self-play-icon {
    font-size: 1.3rem;
  }

  .self-play-config {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .config-label {
    color: #ccc;
    margin: 0;
    font-size: 1rem;
  }

  .game-count-input {
    width: 120px;
    padding: 8px 12px;
    border-radius: 8px;
    border: 1px solid #555;
    background-color: #2a2a2a;
    color: #e0e0e0;
    font-size: 1.2rem;
    text-align: center;
  }

  .game-count-input:focus {
    outline: none;
    border-color: #7b61ff;
  }

  .config-buttons {
    display: flex;
    gap: 16px;
    margin-top: 8px;
  }

  .action-btn {
    padding: 10px 28px;
    border-radius: 8px;
    border: 1px solid #555;
    font-size: 1rem;
    cursor: pointer;
    transition:
      border-color 0.2s,
      background-color 0.2s;
    color: #e0e0e0;
  }

  .start-btn {
    background-color: #2d5a27;
    border-color: #3d7a37;
  }

  .start-btn:hover {
    background-color: #3d7a37;
  }

  .back-btn {
    background-color: #2a2a2a;
  }

  .back-btn:hover {
    border-color: #7b61ff;
    background-color: #333;
  }
</style>
