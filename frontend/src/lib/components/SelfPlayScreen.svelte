<script lang="ts">
  import { onMount } from 'svelte';
  import { Chessground } from '@lichess-org/chessground';
  import type { Api } from '@lichess-org/chessground/api';
  import type { Key, Color } from '@lichess-org/chessground/types';
  import { checkColor } from '../chess/board';
  import { getGameState, cancelSelfPlay, returnFromSelfPlay } from '../state/game.svelte';

  const game = getGameState();

  let isComplete = $derived(game.selfPlayGamesCompleted >= game.selfPlayGamesTotal && game.selfPlayGamesTotal > 0);
  let progressPct = $derived(
    game.selfPlayGamesTotal > 0
      ? Math.round((game.selfPlayGamesCompleted / game.selfPlayGamesTotal) * 100)
      : 0
  );

  let boardEl: HTMLDivElement;
  let cg: Api | undefined;

  onMount(() => {
    cg = Chessground(boardEl, {
      fen: game.selfPlayFen,
      orientation: 'white' as Color,
      viewOnly: true,
      animation: {
        enabled: true,
        duration: 150,
      },
      highlight: {
        lastMove: true,
        check: true,
      },
      movable: {
        free: false,
        color: undefined,
        dests: new Map(),
      },
      draggable: {
        enabled: false,
      },
      premovable: {
        enabled: false,
      },
    });

    return () => {
      cg?.destroy();
    };
  });

  // Reactively update the board when self-play state changes
  $effect(() => {
    if (!cg) return;

    const check = checkColor(game.selfPlayFen);
    const lastMove = game.selfPlayLastMove;

    cg.set({
      fen: game.selfPlayFen,
      check: check as Color | boolean | undefined,
      lastMove: lastMove ? [lastMove.from as Key, lastMove.to as Key] : undefined,
    });
  });
</script>

<div class="self-play-screen">
  <h2>Self-Play Mode</h2>

  <div class="main-layout">
    <div class="board-section">
      <div class="board-container">
        <div class="board-wrap cg-wrap" bind:this={boardEl}></div>
      </div>
      {#if game.selfPlayCurrentGame > 0}
        <p class="board-info">
          Game {game.selfPlayCurrentGame} &middot; Move {game.selfPlayMoveNumber}
        </p>
      {/if}
    </div>

    <div class="side-panel">
      <div class="progress-section">
        <div class="progress-bar-container">
          <div class="progress-bar" style="width: {progressPct}%"></div>
        </div>
        <p class="progress-text">
          {game.selfPlayGamesCompleted} / {game.selfPlayGamesTotal} games
          {#if isComplete}
            — Complete
          {/if}
        </p>
      </div>

      <div class="stats">
        <div class="stat">
          <span class="stat-label">White wins</span>
          <span class="stat-value white-val">{game.selfPlayWhiteWins}</span>
        </div>
        <div class="stat">
          <span class="stat-label">Black wins</span>
          <span class="stat-value black-val">{game.selfPlayBlackWins}</span>
        </div>
        <div class="stat">
          <span class="stat-label">Draws</span>
          <span class="stat-value draw-val">{game.selfPlayDraws}</span>
        </div>
        <div class="stat">
          <span class="stat-label">Weight version</span>
          <span class="stat-value">{game.selfPlayWeightVersion}</span>
        </div>
      </div>

      {#if game.selfPlayGameLog.length > 0}
        <div class="game-log">
          <table>
            <thead>
              <tr>
                <th>#</th>
                <th>Result</th>
                <th>Reason</th>
                <th>Moves</th>
                <th>TD Error</th>
                <th>Weights</th>
              </tr>
            </thead>
            <tbody>
              {#each game.selfPlayGameLog as entry}
                <tr>
                  <td>{entry.gameNumber}</td>
                  <td class="result-cell {entry.result}">{entry.result}</td>
                  <td>{entry.reason}</td>
                  <td>{entry.moves}</td>
                  <td>{entry.avgTdError.toFixed(4)}</td>
                  <td>v{entry.weightVersion}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      <div class="actions">
        {#if isComplete}
          <button class="action-btn back-btn" onclick={returnFromSelfPlay}>
            Back to Menu
          </button>
        {:else}
          <button class="action-btn cancel-btn" onclick={cancelSelfPlay}>
            Cancel
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .self-play-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
    max-width: 900px;
    margin: 0 auto;
    gap: 16px;
  }

  h2 {
    color: #e0e0e0;
    margin: 0;
  }

  .main-layout {
    display: flex;
    gap: 20px;
    align-items: flex-start;
    width: 100%;
  }

  .board-section {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .board-container {
    width: 400px;
    height: 400px;
  }

  .board-wrap {
    width: 100%;
    height: 100%;
  }

  .board-info {
    color: #aaa;
    font-size: 0.85rem;
    margin: 0;
  }

  .side-panel {
    flex: 1;
    min-width: 240px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .progress-section {
    width: 100%;
  }

  .progress-bar-container {
    width: 100%;
    height: 12px;
    background-color: #333;
    border-radius: 6px;
    overflow: hidden;
  }

  .progress-bar {
    height: 100%;
    background-color: #7b61ff;
    border-radius: 6px;
    transition: width 0.3s ease;
  }

  .progress-text {
    text-align: center;
    color: #aaa;
    margin: 8px 0 0;
    font-size: 0.9rem;
  }

  .stats {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    justify-content: center;
  }

  .stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 10px 14px;
    background-color: #2a2a2a;
    border-radius: 8px;
    border: 1px solid #444;
    min-width: 80px;
  }

  .stat-label {
    color: #888;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .stat-value {
    font-size: 1.2rem;
    font-weight: 600;
    color: #e0e0e0;
  }

  .white-val { color: #f0d9b5; }
  .black-val { color: #b58863; }
  .draw-val { color: #888; }

  .game-log {
    width: 100%;
    max-height: 250px;
    overflow-y: auto;
    border: 1px solid #444;
    border-radius: 8px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  thead {
    position: sticky;
    top: 0;
  }

  th {
    padding: 8px 12px;
    text-align: left;
    background-color: #333;
    color: #aaa;
    font-weight: 600;
    text-transform: uppercase;
    font-size: 0.75rem;
    letter-spacing: 0.5px;
  }

  td {
    padding: 6px 12px;
    border-top: 1px solid #333;
    color: #ccc;
  }

  .result-cell.white { color: #f0d9b5; }
  .result-cell.black { color: #b58863; }
  .result-cell.draw { color: #888; }

  .actions {
    margin-top: 8px;
    text-align: center;
  }

  .action-btn {
    padding: 10px 32px;
    border-radius: 8px;
    border: 1px solid #555;
    font-size: 1rem;
    cursor: pointer;
    color: #e0e0e0;
    transition:
      border-color 0.2s,
      background-color 0.2s;
  }

  .cancel-btn {
    background-color: #5a2727;
    border-color: #7a3737;
  }

  .cancel-btn:hover {
    background-color: #7a3737;
  }

  .back-btn {
    background-color: #2a2a2a;
  }

  .back-btn:hover {
    border-color: #7b61ff;
    background-color: #333;
  }
</style>
