import { wsClient } from '../ws/client';
import type { LearningUpdateMessage } from '../ws/protocol';

export interface LearningMetrics {
  gameId: number;
  avgTdError: number;
  maxTdError: number;
  weightChangeNorm: number;
  weightVersion: number;
}

let lastGameMetrics = $state<LearningMetrics | null>(null);
let gamesPlayed = $state(0);

export function initLearningState(): void {
  wsClient.onMessage('learning_update', (msg) => {
    const data = msg as LearningUpdateMessage;
    lastGameMetrics = {
      gameId: data.game_id,
      avgTdError: data.avg_td_error,
      maxTdError: data.max_td_error,
      weightChangeNorm: data.weight_change_norm,
      weightVersion: data.weight_version,
    };
    gamesPlayed += 1;
  });

  wsClient.onMessage('game_started', () => {
    lastGameMetrics = null;
  });
}

export function getLearningState() {
  return {
    get lastGameMetrics() {
      return lastGameMetrics;
    },
    get gamesPlayed() {
      return gamesPlayed;
    },
  };
}
