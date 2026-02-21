<script lang="ts">
  import ChessBoard from './ChessBoard.svelte';
  import GameStatusBar from './GameStatusBar.svelte';
  import SearchPanel from './SearchPanel.svelte';
  import MoveList from './MoveList.svelte';
  import GameControls from './GameControls.svelte';
  import LearningDashboard from './LearningDashboard.svelte';
</script>

<div class="game-screen">
  <GameStatusBar />
  <div class="game-layout">
    <div class="board-section">
      <ChessBoard />
    </div>
    <div class="side-panel">
      <SearchPanel />
      <MoveList />
      <GameControls />
      <LearningDashboard />
    </div>
  </div>
</div>

<style>
  .game-screen {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
    max-width: 900px;
    margin: 0 auto;
  }

  .game-layout {
    display: flex;
    gap: 20px;
    align-items: flex-start;
  }

  .board-section {
    flex-shrink: 0;
  }

  .side-panel {
    flex: 1;
    min-width: 240px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
</style>
