<script lang="ts">
  import { onMount } from 'svelte';
  import { Chessground } from '@lichess-org/chessground';
  import type { Api } from '@lichess-org/chessground/api';
  import type { Key, Color } from '@lichess-org/chessground/types';
  import { legalDests, isPromotion, checkColor } from '../chess/board';
  import { getGameState, makeMove } from '../state/game.svelte';

  const game = getGameState();
  let boardEl: HTMLDivElement;
  let cg: Api | undefined;

  onMount(() => {
    cg = Chessground(boardEl, {
      fen: game.fen,
      orientation: game.playerColor as Color,
      turnColor: game.turn as Color,
      movable: {
        free: false,
        color: game.playerColor as Color,
        dests: game.turn === game.playerColor ? legalDests(game.fen) : new Map(),
        showDests: true,
        events: {
          after: onUserMove,
        },
      },
      animation: {
        enabled: true,
        duration: 200,
      },
      highlight: {
        lastMove: true,
        check: true,
      },
      premovable: {
        enabled: false,
      },
      draggable: {
        enabled: true,
        showGhost: true,
      },
    });

    return () => {
      cg?.destroy();
    };
  });

  function onUserMove(orig: Key, dest: Key): void {
    const from = orig as string;
    const to = dest as string;

    if (isPromotion(game.fen, from, to)) {
      // Default to queen promotion
      makeMove(from, to, 'q');
    } else {
      makeMove(from, to, null);
    }
  }

  // Reactively update the board when game state changes
  $effect(() => {
    if (!cg) return;

    const isPlayerTurn = game.turn === game.playerColor;
    const check = checkColor(game.fen);

    cg.set({
      fen: game.fen,
      orientation: game.playerColor as Color,
      turnColor: game.turn as Color,
      check: check as Color | boolean | undefined,
      lastMove:
        game.moveHistory.length > 0
          ? [
              game.moveHistory[game.moveHistory.length - 1].from as Key,
              game.moveHistory[game.moveHistory.length - 1].to as Key,
            ]
          : undefined,
      movable: {
        free: false,
        color: game.status === 'playing' ? (game.playerColor as Color) : undefined,
        dests:
          game.status === 'playing' && isPlayerTurn
            ? legalDests(game.fen)
            : new Map(),
        showDests: true,
      },
    });
  });
</script>

<div class="board-container">
  <div class="board-wrap cg-wrap" bind:this={boardEl}></div>
</div>

<style>
  .board-container {
    width: 520px;
    height: 520px;
    flex-shrink: 0;
  }

  .board-wrap {
    width: 100%;
    height: 100%;
  }
</style>
