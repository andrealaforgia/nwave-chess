// Client-to-Server messages

export interface SelectColorMessage {
  type: 'select_color';
  color: 'white' | 'black';
}

export interface MakeMoveMessage {
  type: 'make_move';
  from: string;
  to: string;
  promotion: string | null;
}

export interface ResignMessage {
  type: 'resign';
}

export interface NewGameMessage {
  type: 'new_game';
}

export interface RequestLearningStatusMessage {
  type: 'request_learning_status';
}

export interface StartSelfPlayMessage {
  type: 'start_self_play';
  num_games: number;
}

export interface CancelSelfPlayMessage {
  type: 'cancel_self_play';
}

export type ClientMessage =
  | SelectColorMessage
  | MakeMoveMessage
  | ResignMessage
  | NewGameMessage
  | RequestLearningStatusMessage
  | StartSelfPlayMessage
  | CancelSelfPlayMessage;

// Server-to-Client messages

export interface GameStartedMessage {
  type: 'game_started';
  fen: string;
  player_color: string;
  engine_color: string;
  weight_version: number;
}

export interface MoveAcceptedMessage {
  type: 'move_accepted';
  from: string;
  to: string;
  fen: string;
}

export interface MoveRejectedMessage {
  type: 'move_rejected';
  reason: string;
  fen: string;
}

export interface EngineThinkingMessage {
  type: 'engine_thinking';
  depth: number;
  evaluation_cp: number;
  best_move: string;
  pv_line: string[];
  nodes: number;
  time_ms: number;
}

export interface EngineMoveMessage {
  type: 'engine_move';
  from: string;
  to: string;
  promotion: string | null;
  fen: string;
  evaluation_cp: number;
  search_depth: number;
  nodes_searched: number;
}

export interface GameOverMessage {
  type: 'game_over';
  result: string;
  reason: string;
  fen: string;
}

export interface LearningUpdateMessage {
  type: 'learning_update';
  game_id: number;
  avg_td_error: number;
  max_td_error: number;
  weight_change_norm: number;
  weight_version: number;
}

export interface SelfPlayMoveMessage {
  type: 'self_play_move';
  game_number: number;
  total_games: number;
  move_number: number;
  from: string;
  to: string;
  promotion: string | null;
  fen: string;
}

export interface SelfPlayProgressMessage {
  type: 'self_play_progress';
  game_number: number;
  total_games: number;
  result: string;
  reason: string;
  moves: number;
  avg_td_error: number;
  weight_version: number;
}

export interface SelfPlayCompleteMessage {
  type: 'self_play_complete';
  total_games: number;
  white_wins: number;
  black_wins: number;
  draws: number;
  weight_version: number;
}

export interface ErrorMessage {
  type: 'error';
  code: string;
  message: string;
}

export type ServerMessage =
  | GameStartedMessage
  | MoveAcceptedMessage
  | MoveRejectedMessage
  | EngineThinkingMessage
  | EngineMoveMessage
  | GameOverMessage
  | LearningUpdateMessage
  | SelfPlayMoveMessage
  | SelfPlayProgressMessage
  | SelfPlayCompleteMessage
  | ErrorMessage;

export type ServerMessageType = ServerMessage['type'];
