<script lang="ts">
  import { getGameState, resign, newGame } from '../state/game.svelte';

  const game = getGameState();
</script>

<div class="game-controls">
  {#if game.status === 'playing'}
    <button class="btn resign-btn" onclick={() => resign()}>
      Resign
    </button>
  {/if}
  {#if game.status === 'game_over'}
    <button class="btn new-game-btn" onclick={() => newGame()}>
      New Game
    </button>
  {/if}
  {#if game.lastMoveRejectedReason}
    <span class="error-msg">{game.lastMoveRejectedReason}</span>
  {/if}
</div>

<style>
  .game-controls {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .btn {
    padding: 8px 20px;
    border-radius: 6px;
    border: 1px solid #444;
    font-size: 0.9rem;
    cursor: pointer;
    transition:
      background-color 0.2s,
      border-color 0.2s;
    background-color: #2a2a2a;
    color: #e0e0e0;
  }

  .btn:hover {
    border-color: #666;
    background-color: #333;
  }

  .resign-btn:hover {
    border-color: #f44336;
    color: #f44336;
  }

  .new-game-btn:hover {
    border-color: #7b61ff;
    color: #7b61ff;
  }

  .error-msg {
    font-size: 0.8rem;
    color: #f44336;
  }
</style>
