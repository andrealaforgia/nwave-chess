<script lang="ts">
  import { getSearchState } from '../state/search.svelte';

  const search = getSearchState();

  const evalDisplay = $derived.by(() => {
    const cp = search.evaluation;
    const pawns = cp / 100;
    const sign = pawns >= 0 ? '+' : '';
    return `${sign}${pawns.toFixed(2)}`;
  });

  const nodesDisplay = $derived.by(() => {
    const n = search.nodeCount;
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
    return `${n}`;
  });

  const timeDisplay = $derived.by(() => {
    const ms = search.timeMs;
    if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
    return `${ms}ms`;
  });
</script>

{#if search.isSearching}
  <div class="search-panel">
    <div class="panel-header">
      <span class="title">Engine Analysis</span>
      <span class="thinking-indicator">Thinking...</span>
    </div>
    <div class="stats-row">
      <div class="stat">
        <span class="stat-label">Eval</span>
        <span class="stat-value eval" class:positive={search.evaluation >= 0} class:negative={search.evaluation < 0}>
          {evalDisplay}
        </span>
      </div>
      <div class="stat">
        <span class="stat-label">Depth</span>
        <span class="stat-value">{search.currentDepth}</span>
      </div>
      <div class="stat">
        <span class="stat-label">Nodes</span>
        <span class="stat-value">{nodesDisplay}</span>
      </div>
      <div class="stat">
        <span class="stat-label">Time</span>
        <span class="stat-value">{timeDisplay}</span>
      </div>
    </div>
    {#if search.pvLine.length > 0}
      <div class="pv-line">
        <span class="pv-label">PV:</span>
        <span class="pv-moves">{search.pvLine.join(' ')}</span>
      </div>
    {/if}
    {#if search.bestMove}
      <div class="best-move">
        Best: <strong>{search.bestMove}</strong>
      </div>
    {/if}
  </div>
{/if}

<style>
  .search-panel {
    background-color: #1e1e1e;
    border: 1px solid #333;
    border-radius: 8px;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .title {
    font-size: 0.85rem;
    font-weight: 600;
    color: #aaa;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .thinking-indicator {
    font-size: 0.75rem;
    color: #7b61ff;
    animation: pulse 1.5s infinite;
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

  .stats-row {
    display: flex;
    gap: 16px;
  }

  .stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .stat-label {
    font-size: 0.7rem;
    color: #666;
    text-transform: uppercase;
  }

  .stat-value {
    font-size: 1rem;
    font-weight: 600;
    color: #e0e0e0;
    font-variant-numeric: tabular-nums;
  }

  .eval.positive {
    color: #4caf50;
  }

  .eval.negative {
    color: #f44336;
  }

  .pv-line {
    font-size: 0.8rem;
    color: #999;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .pv-label {
    color: #666;
    margin-right: 4px;
  }

  .pv-moves {
    font-family: 'SF Mono', 'Menlo', 'Monaco', monospace;
  }

  .best-move {
    font-size: 0.8rem;
    color: #aaa;
  }

  .best-move strong {
    color: #7b61ff;
    font-family: 'SF Mono', 'Menlo', 'Monaco', monospace;
  }
</style>
