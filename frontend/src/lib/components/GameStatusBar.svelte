<script lang="ts">
  import { getGameState } from '../state/game.svelte';
  import { isInCheck } from '../chess/board';

  const game = getGameState();

  const statusText = $derived.by(() => {
    if (game.status === 'game_over') {
      return formatResult(game.result, game.reason);
    }

    const turnLabel = game.turn === 'white' ? 'White' : 'Black';
    const isYourTurn = game.turn === game.playerColor;
    const check = isInCheck(game.fen);

    let text = `${turnLabel} to move`;
    if (isYourTurn) {
      text += ' (your turn)';
    } else {
      text += ' (engine thinking)';
    }
    if (check) {
      text += ' - CHECK';
    }
    return text;
  });

  function formatResult(result: string, reason: string): string {
    let winner = '';
    if (result === 'white') winner = 'White wins';
    else if (result === 'black') winner = 'Black wins';
    else winner = 'Draw';

    const reasonLabel = reason
      .split('_')
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(' ');

    return `${winner} - ${reasonLabel}`;
  }
</script>

<div class="status-bar" class:game-over={game.status === 'game_over'}>
  <span class="status-text">{statusText}</span>
  {#if game.weightVersion > 0}
    <span class="weight-version">Weights v{game.weightVersion}</span>
  {/if}
</div>

<style>
  .status-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 16px;
    background-color: #2a2a2a;
    border-radius: 6px;
    font-size: 0.9rem;
    color: #ccc;
  }

  .status-bar.game-over {
    background-color: #3a2a1a;
    color: #ffcc80;
  }

  .weight-version {
    font-size: 0.75rem;
    color: #777;
  }
</style>
