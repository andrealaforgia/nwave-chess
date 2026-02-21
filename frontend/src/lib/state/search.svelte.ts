import { wsClient } from '../ws/client';
import type { EngineThinkingMessage } from '../ws/protocol';

let currentDepth = $state(0);
let bestMove = $state('');
let evaluation = $state(0);
let pvLine = $state<string[]>([]);
let nodeCount = $state(0);
let isSearching = $state(false);
let timeMs = $state(0);

export function initSearchState(): void {
  wsClient.onMessage('engine_thinking', (msg) => {
    const data = msg as EngineThinkingMessage;
    currentDepth = data.depth;
    bestMove = data.best_move;
    evaluation = data.evaluation_cp;
    pvLine = data.pv_line;
    nodeCount = data.nodes;
    isSearching = true;
    timeMs = data.time_ms;
  });

  wsClient.onMessage('engine_move', () => {
    isSearching = false;
  });

  wsClient.onMessage('game_started', () => {
    resetSearch();
  });
}

function resetSearch(): void {
  currentDepth = 0;
  bestMove = '';
  evaluation = 0;
  pvLine = [];
  nodeCount = 0;
  isSearching = false;
  timeMs = 0;
}

export function getSearchState() {
  return {
    get currentDepth() {
      return currentDepth;
    },
    get bestMove() {
      return bestMove;
    },
    get evaluation() {
      return evaluation;
    },
    get pvLine() {
      return pvLine;
    },
    get nodeCount() {
      return nodeCount;
    },
    get isSearching() {
      return isSearching;
    },
    get timeMs() {
      return timeMs;
    },
  };
}
