import { Chess } from 'chess.js';
import type { Key } from '@lichess-org/chessground/types';

/**
 * Compute the legal move destinations map for chessground from a FEN position.
 * Returns a Map where keys are origin squares and values are arrays of destination squares.
 */
export function legalDests(fen: string): Map<Key, Key[]> {
  const chess = new Chess(fen);
  const dests = new Map<Key, Key[]>();

  const moves = chess.moves({ verbose: true });
  for (const move of moves) {
    const from = move.from as Key;
    const existing = dests.get(from);
    if (existing) {
      existing.push(move.to as Key);
    } else {
      dests.set(from, [move.to as Key]);
    }
  }

  return dests;
}

/**
 * Check if a move is a pawn promotion.
 */
export function isPromotion(fen: string, from: string, to: string): boolean {
  const chess = new Chess(fen);
  const moves = chess.moves({ verbose: true });
  return moves.some(
    (m) => m.from === from && m.to === to && m.promotion !== undefined
  );
}

/**
 * Determine if the current position is in check.
 */
export function isInCheck(fen: string): boolean {
  const chess = new Chess(fen);
  return chess.isCheck();
}

/**
 * Get the color whose king is in check, or null.
 */
export function checkColor(fen: string): 'white' | 'black' | undefined {
  const chess = new Chess(fen);
  if (!chess.isCheck()) return undefined;
  return chess.turn() === 'w' ? 'white' : 'black';
}
