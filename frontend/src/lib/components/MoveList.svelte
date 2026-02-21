<script lang="ts">
  import { getGameState } from '../state/game.svelte';

  const game = getGameState();

  // Group moves into pairs (white move + black move)
  const movePairs = $derived.by(() => {
    const pairs: { number: number; white: string; black: string }[] = [];
    const moves = game.moveHistory;
    for (let i = 0; i < moves.length; i += 2) {
      pairs.push({
        number: Math.floor(i / 2) + 1,
        white: moves[i].san,
        black: i + 1 < moves.length ? moves[i + 1].san : '',
      });
    }
    return pairs;
  });

  let listEl: HTMLDivElement | undefined;

  // Auto-scroll to bottom when new moves arrive
  $effect(() => {
    // Access moveHistory length to create dependency
    const _len = game.moveHistory.length;
    if (listEl) {
      // Use void to suppress unused var warning
      void _len;
      requestAnimationFrame(() => {
        if (listEl) {
          listEl.scrollTop = listEl.scrollHeight;
        }
      });
    }
  });
</script>

<div class="move-list" bind:this={listEl}>
  {#if movePairs.length === 0}
    <div class="empty">No moves yet</div>
  {:else}
    {#each movePairs as pair}
      <div class="move-row">
        <span class="move-number">{pair.number}.</span>
        <span class="move white-move">{pair.white}</span>
        {#if pair.black}
          <span class="move black-move">{pair.black}</span>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .move-list {
    background-color: #1e1e1e;
    border: 1px solid #333;
    border-radius: 8px;
    padding: 8px;
    max-height: 240px;
    overflow-y: auto;
    font-family: 'SF Mono', 'Menlo', 'Monaco', monospace;
    font-size: 0.85rem;
  }

  .empty {
    color: #555;
    text-align: center;
    padding: 16px;
    font-family: inherit;
  }

  .move-row {
    display: flex;
    gap: 8px;
    padding: 2px 4px;
    border-radius: 3px;
  }

  .move-row:hover {
    background-color: #2a2a2a;
  }

  .move-number {
    color: #555;
    min-width: 28px;
    text-align: right;
  }

  .move {
    color: #ccc;
    min-width: 56px;
  }

  .move:hover {
    color: #7b61ff;
    cursor: default;
  }
</style>
