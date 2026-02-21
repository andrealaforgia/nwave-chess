<script lang="ts">
  import { getLearningState } from '../state/learning.svelte';

  const learning = getLearningState();
</script>

{#if learning.lastGameMetrics}
  <div class="learning-dashboard">
    <div class="dashboard-header">
      <span class="title">Learning Update</span>
      <span class="games-count">Games: {learning.gamesPlayed}</span>
    </div>
    <div class="metrics-grid">
      <div class="metric">
        <span class="metric-label">Avg TD Error</span>
        <span class="metric-value">{learning.lastGameMetrics.avgTdError.toFixed(4)}</span>
      </div>
      <div class="metric">
        <span class="metric-label">Max TD Error</span>
        <span class="metric-value">{learning.lastGameMetrics.maxTdError.toFixed(4)}</span>
      </div>
      <div class="metric">
        <span class="metric-label">Weight Change</span>
        <span class="metric-value">{learning.lastGameMetrics.weightChangeNorm.toFixed(4)}</span>
      </div>
      <div class="metric">
        <span class="metric-label">Weight Version</span>
        <span class="metric-value">v{learning.lastGameMetrics.weightVersion}</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .learning-dashboard {
    background-color: #1a2a1a;
    border: 1px solid #2d4a2d;
    border-radius: 8px;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .dashboard-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .title {
    font-size: 0.85rem;
    font-weight: 600;
    color: #66bb6a;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .games-count {
    font-size: 0.75rem;
    color: #4caf50;
  }

  .metrics-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .metric {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 8px;
    background-color: #1e2e1e;
    border-radius: 4px;
  }

  .metric-label {
    font-size: 0.7rem;
    color: #5a8a5a;
    text-transform: uppercase;
  }

  .metric-value {
    font-size: 0.95rem;
    font-weight: 600;
    color: #c8e6c9;
    font-variant-numeric: tabular-nums;
    font-family: 'SF Mono', 'Menlo', 'Monaco', monospace;
  }
</style>
