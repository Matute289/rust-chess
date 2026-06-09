use crate::eval::evaluate;
use crate::moves::Move;
use crate::position::Position;
use crate::types::PieceType;

// ── Constants ─────────────────────────────────────────────────────────────────

pub const MATE_SCORE: i32 = 100_000;
pub const DRAW_SCORE: i32 = 0;
const INF: i32 = i32::MAX / 2;

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct DifficultyConfig {
    pub max_depth:     u8,
    pub max_nodes:     u64,   // node budget — platform-agnostic time proxy
    pub random_factor: f32,   // 0.0 = best move; >0 = pick randomly among near-best moves
}

impl DifficultyConfig {
    pub fn principiante() -> Self { DifficultyConfig { max_depth: 2, max_nodes: 50_000,     random_factor: 0.25 } }
    pub fn facil()         -> Self { DifficultyConfig { max_depth: 3, max_nodes: 200_000,    random_factor: 0.10 } }
    pub fn medio()         -> Self { DifficultyConfig { max_depth: 5, max_nodes: 2_000_000,  random_factor: 0.00 } }
    pub fn dificil()       -> Self { DifficultyConfig { max_depth: 7, max_nodes: 10_000_000, random_factor: 0.00 } }
    pub fn pro()           -> Self { DifficultyConfig { max_depth: 64, max_nodes: 50_000_000, random_factor: 0.00 } }
}

#[derive(Debug)]
pub enum SearchResult {
    EngineMove(Move, i32),  // (best move, centipawn score from side-to-move perspective)
}

// ── Search state ──────────────────────────────────────────────────────────────

pub struct Search {
    pub nodes: u64,
}

impl Search {
    pub fn new() -> Search {
        Search { nodes: 0 }
    }

    /// Returns the best move for the given position and difficulty.
    pub fn best_move(&mut self, pos: &Position, config: &DifficultyConfig) -> SearchResult {
        self.nodes = 0;
        let moves = pos.legal_moves();

        if moves.is_empty() {
            return SearchResult::EngineMove(
                Move::NULL,
                if pos.is_in_check() { -MATE_SCORE } else { DRAW_SCORE },
            );
        }

        let mut best       = moves[0];
        let mut best_score = -INF;

        for &m in &moves {
            let child = pos.make_move(m);
            let score = -self.negamax(&child, config.max_depth - 1, -INF, INF, config.max_nodes);
            if score > best_score {
                best_score = score;
                best = m;
            }
        }

        SearchResult::EngineMove(best, best_score)
    }

    fn negamax(&mut self, pos: &Position, depth: u8, mut alpha: i32, beta: i32, max_nodes: u64) -> i32 {
        self.nodes += 1;

        // If the current side's king is gone (captured by a pseudo-legal move), that's an
        // illegal state — score as a decisive loss for the side to move.
        let us = pos.side_to_move as usize;
        if pos.pieces[us][PieceType::King as usize].0 == 0 {
            return -MATE_SCORE;
        }

        if self.nodes >= max_nodes { return evaluate(pos); }
        if depth == 0              { return evaluate(pos); }

        let moves = pos.legal_moves();
        if moves.is_empty() {
            return if pos.is_in_check() {
                // Checkmate — return negative mate score, offset by depth to prefer shorter mates
                -(MATE_SCORE - depth as i32)
            } else {
                DRAW_SCORE
            };
        }

        for m in moves {
            let child = pos.make_move(m);
            let score = -self.negamax(&child, depth - 1, -beta, -alpha, max_nodes);
            if score >= beta { return beta; }  // beta cutoff
            if score > alpha { alpha = score; }
        }

        alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;

    fn pos(fen: &str) -> Position { Position::from_fen(fen).unwrap() }

    #[test]
    fn mate_in_one_rook() {
        // White rook on a7, white king on b6, black king on a8 — Ra8#
        let p = pos("k7/R7/1K6/8/8/8/8/8 w - - 0 1");
        let config = DifficultyConfig::medio();
        let result = Search::new().best_move(&p, &config);
        if let SearchResult::EngineMove(m, score) = result {
            assert!(score >= MATE_SCORE - 100, "score {} should be near mate", score);
            assert_eq!(m.to_uci(), "a7a8", "expected Ra8# got {}", m.to_uci());
        } else {
            panic!("expected EngineMove");
        }
    }

    #[test]
    fn mate_in_one_queen() {
        // White queen on b1, white king on a3, black king on a8 — Qb8#
        let p = pos("k7/8/K7/8/8/8/8/1Q6 w - - 0 1");
        let config = DifficultyConfig::medio();
        let result = Search::new().best_move(&p, &config);
        if let SearchResult::EngineMove(m, score) = result {
            assert!(score >= MATE_SCORE - 100, "score {} should be near mate", score);
        } else {
            panic!("expected EngineMove");
        }
    }

    #[test]
    fn returns_a_move() {
        let p = pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let config = DifficultyConfig::facil();
        let result = Search::new().best_move(&p, &config);
        assert!(matches!(result, SearchResult::EngineMove(_, _)));
    }
}
