import { Chess } from 'chess.js';
import { wsClient } from '../ws/client';
import type {
  GameStartedMessage,
  MoveAcceptedMessage,
  MoveRejectedMessage,
  EngineMoveMessage,
  GameOverMessage,
  SelfPlayMoveMessage,
  SelfPlayProgressMessage,
  SelfPlayCompleteMessage,
} from '../ws/protocol';

export type GameStatus =
  | 'selecting_color'
  | 'playing'
  | 'game_over'
  | 'self_play_config'
  | 'self_play_running';

export interface MoveRecord {
  from: string;
  to: string;
  san: string;
  fen: string;
}

export interface SelfPlayGameRecord {
  gameNumber: number;
  result: string;
  reason: string;
  moves: number;
  avgTdError: number;
  weightVersion: number;
}

const STARTING_FEN = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';

let fen = $state(STARTING_FEN);
let playerColor = $state<'white' | 'black'>('white');
let moveHistory = $state<MoveRecord[]>([]);
let result = $state('');
let reason = $state('');
let status = $state<GameStatus>('selecting_color');
let weightVersion = $state(0);
let lastMoveRejectedReason = $state('');

// Self-play state
let selfPlayGamesCompleted = $state(0);
let selfPlayGamesTotal = $state(0);
let selfPlayWhiteWins = $state(0);
let selfPlayBlackWins = $state(0);
let selfPlayDraws = $state(0);
let selfPlayGameLog = $state<SelfPlayGameRecord[]>([]);
let selfPlayWeightVersion = $state(0);

// Self-play live board state
let selfPlayFen = $state(STARTING_FEN);
let selfPlayLastMove = $state<{ from: string; to: string } | null>(null);
let selfPlayCurrentGame = $state(0);
let selfPlayMoveNumber = $state(0);

function turnFromFen(f: string): 'white' | 'black' {
  const parts = f.split(' ');
  return parts[1] === 'b' ? 'black' : 'white';
}

function computeSan(prevFen: string, from: string, to: string, promotion?: string | null): string {
  try {
    const chess = new Chess(prevFen);
    const move = chess.move({ from, to, promotion: promotion ?? undefined });
    return move.san;
  } catch {
    return `${from}-${to}`;
  }
}

export function initGameState(): void {
  wsClient.onMessage('game_started', (msg) => {
    const data = msg as GameStartedMessage;
    fen = data.fen;
    playerColor = data.player_color as 'white' | 'black';
    moveHistory = [];
    result = '';
    reason = '';
    status = 'playing';
    weightVersion = data.weight_version;
    lastMoveRejectedReason = '';
  });

  wsClient.onMessage('move_accepted', (msg) => {
    const data = msg as MoveAcceptedMessage;
    const san = computeSan(fen, data.from, data.to);
    fen = data.fen;
    moveHistory = [...moveHistory, { from: data.from, to: data.to, san, fen: data.fen }];
    lastMoveRejectedReason = '';
  });

  wsClient.onMessage('move_rejected', (msg) => {
    const data = msg as MoveRejectedMessage;
    fen = data.fen;
    lastMoveRejectedReason = data.reason;
  });

  wsClient.onMessage('engine_move', (msg) => {
    const data = msg as EngineMoveMessage;
    const san = computeSan(fen, data.from, data.to, data.promotion);
    fen = data.fen;
    moveHistory = [
      ...moveHistory,
      { from: data.from, to: data.to, san, fen: data.fen },
    ];
  });

  wsClient.onMessage('game_over', (msg) => {
    const data = msg as GameOverMessage;
    fen = data.fen;
    result = data.result;
    reason = data.reason;
    status = 'game_over';
  });

  wsClient.onMessage('self_play_move', (msg) => {
    const data = msg as SelfPlayMoveMessage;
    selfPlayFen = data.fen;
    selfPlayLastMove = { from: data.from, to: data.to };
    selfPlayCurrentGame = data.game_number;
    selfPlayMoveNumber = data.move_number;
  });

  wsClient.onMessage('self_play_progress', (msg) => {
    const data = msg as SelfPlayProgressMessage;
    selfPlayGamesCompleted = data.game_number;
    selfPlayWeightVersion = data.weight_version;

    // Update running totals from individual results.
    if (data.result === 'white') selfPlayWhiteWins++;
    else if (data.result === 'black') selfPlayBlackWins++;
    else selfPlayDraws++;

    selfPlayGameLog = [
      ...selfPlayGameLog,
      {
        gameNumber: data.game_number,
        result: data.result,
        reason: data.reason,
        moves: data.moves,
        avgTdError: data.avg_td_error,
        weightVersion: data.weight_version,
      },
    ];
  });

  wsClient.onMessage('self_play_complete', (msg) => {
    const data = msg as SelfPlayCompleteMessage;
    selfPlayGamesCompleted = data.total_games;
    selfPlayWhiteWins = data.white_wins;
    selfPlayBlackWins = data.black_wins;
    selfPlayDraws = data.draws;
    selfPlayWeightVersion = data.weight_version;
    // Stay on self_play_running screen to show final results.
    // User clicks "Back" to return to color selection.
  });
}

export function selectColor(color: 'white' | 'black'): void {
  wsClient.send({ type: 'select_color', color });
}

export function makeMove(from: string, to: string, promotion: string | null): void {
  wsClient.send({ type: 'make_move', from, to, promotion });
}

export function resign(): void {
  wsClient.send({ type: 'resign' });
}

export function newGame(): void {
  status = 'selecting_color';
  fen = STARTING_FEN;
  moveHistory = [];
  result = '';
  reason = '';
  lastMoveRejectedReason = '';
  wsClient.send({ type: 'new_game' });
}

export function enterSelfPlayConfig(): void {
  status = 'self_play_config';
}

export function exitSelfPlayConfig(): void {
  status = 'selecting_color';
}

export function startSelfPlay(numGames: number): void {
  selfPlayGamesCompleted = 0;
  selfPlayGamesTotal = numGames;
  selfPlayWhiteWins = 0;
  selfPlayBlackWins = 0;
  selfPlayDraws = 0;
  selfPlayGameLog = [];
  selfPlayWeightVersion = 0;
  selfPlayFen = STARTING_FEN;
  selfPlayLastMove = null;
  selfPlayCurrentGame = 0;
  selfPlayMoveNumber = 0;
  status = 'self_play_running';
  wsClient.send({ type: 'start_self_play', num_games: numGames });
}

export function cancelSelfPlay(): void {
  wsClient.send({ type: 'cancel_self_play' });
}

export function returnFromSelfPlay(): void {
  status = 'selecting_color';
  wsClient.send({ type: 'new_game' });
}

export function getGameState() {
  return {
    get fen() {
      return fen;
    },
    get turn() {
      return turnFromFen(fen);
    },
    get playerColor() {
      return playerColor;
    },
    get moveHistory() {
      return moveHistory;
    },
    get result() {
      return result;
    },
    get reason() {
      return reason;
    },
    get status() {
      return status;
    },
    get weightVersion() {
      return weightVersion;
    },
    get lastMoveRejectedReason() {
      return lastMoveRejectedReason;
    },
    get selfPlayGamesCompleted() {
      return selfPlayGamesCompleted;
    },
    get selfPlayGamesTotal() {
      return selfPlayGamesTotal;
    },
    get selfPlayWhiteWins() {
      return selfPlayWhiteWins;
    },
    get selfPlayBlackWins() {
      return selfPlayBlackWins;
    },
    get selfPlayDraws() {
      return selfPlayDraws;
    },
    get selfPlayGameLog() {
      return selfPlayGameLog;
    },
    get selfPlayWeightVersion() {
      return selfPlayWeightVersion;
    },
    get selfPlayFen() {
      return selfPlayFen;
    },
    get selfPlayLastMove() {
      return selfPlayLastMove;
    },
    get selfPlayCurrentGame() {
      return selfPlayCurrentGame;
    },
    get selfPlayMoveNumber() {
      return selfPlayMoveNumber;
    },
  };
}
