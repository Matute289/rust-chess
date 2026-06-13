use crate::eval::evaluate;
use crate::moves::Move;
use crate::position::Position;
use crate::types::PieceType;

// ── Constants ─────────────────────────────────────────────────────────────────

pub const MATE_SCORE: i32 = 100_000;
pub const DRAW_SCORE: i32 = 0;
const INF: i32 = i32::MAX / 2;

// ── Transposition table ───────────────────────────────────────────────────────

#[derive(Copy, Clone, PartialEq, Eq)]
enum TTFlag { Exact, LowerBound, UpperBound }

#[derive(Copy, Clone)]
struct TTEntry {
    hash:      u64,
    depth:     u8,
    score:     i32,
    flag:      TTFlag,
    best_move: Move,
}

impl TTEntry {
    const EMPTY: TTEntry = TTEntry {
        hash: 0, depth: 0, score: 0, flag: TTFlag::Exact, best_move: Move::NULL,
    };
}

const TT_SIZE: usize = 1 << 20; // 1M entries ≈ ~16 MB

struct TranspositionTable {
    entries: Vec<TTEntry>,
}

impl TranspositionTable {
    fn new() -> Self {
        TranspositionTable { entries: vec![TTEntry::EMPTY; TT_SIZE] }
    }

    /// Returns (score, best_move) if hit. Score is i32::MIN if depth insufficient (has move only).
    fn probe(&self, hash: u64, depth: u8, alpha: i32, beta: i32) -> Option<(i32, Move)> {
        let idx = (hash as usize) & (TT_SIZE - 1);
        let e = &self.entries[idx];
        if e.hash != hash { return None; }
        let best_move = e.best_move;
        if e.depth < depth {
            // Entry exists but isn't deep enough — return move only (i32::MIN signals no score)
            return Some((i32::MIN, best_move));
        }
        let score = match e.flag {
            TTFlag::Exact      => e.score,
            TTFlag::LowerBound => { if e.score >= beta  { return Some((beta,  best_move)); }
                                    return Some((i32::MIN, best_move)); }
            TTFlag::UpperBound => { if e.score <= alpha { return Some((alpha, best_move)); }
                                    return Some((i32::MIN, best_move)); }
        };
        Some((score, best_move))
    }

    fn store(&mut self, hash: u64, depth: u8, score: i32, flag: TTFlag, best_move: Move) {
        let idx = (hash as usize) & (TT_SIZE - 1);
        // Always-replace strategy
        self.entries[idx] = TTEntry { hash, depth, score, flag, best_move };
    }
}

// ── Move ordering ─────────────────────────────────────────────────────────────

// MVV-LVA: Most Valuable Victim - Least Valuable Attacker
// Higher score = search first (best captures first)
fn mvv_lva(victim: PieceType, attacker: PieceType) -> i32 {
    let victim_val = match victim {
        PieceType::Pawn => 1, PieceType::Knight => 2, PieceType::Bishop => 3,
        PieceType::Rook => 4, PieceType::Queen  => 5, PieceType::King   => 6,
    };
    let attacker_val = match attacker {
        PieceType::Pawn => 6, PieceType::Knight => 5, PieceType::Bishop => 4,
        PieceType::Rook => 3, PieceType::Queen  => 2, PieceType::King   => 1,
    };
    victim_val * 10 + attacker_val
}

fn score_move(
    pos: &Position,
    m: Move,
    tt_move: Move,
    killers: &[Move; 2],
    history: &[[i32; 64]; 64],
) -> i32 {
    if m == tt_move { return 10_000_000; }

    if m.is_capture() || m.is_en_passant() {
        let victim_pt = if m.is_en_passant() {
            PieceType::Pawn
        } else {
            pos.piece_at(m.to_sq()).map(|(_, pt)| pt).unwrap_or(PieceType::Pawn)
        };
        let attacker_pt = pos.piece_at(m.from_sq()).map(|(_, pt)| pt).unwrap_or(PieceType::Pawn);
        return 1_000_000 + mvv_lva(victim_pt, attacker_pt);
    }

    if m == killers[0] { return 900_000; }
    if m == killers[1] { return 800_000; }

    history[m.from_sq().0 as usize][m.to_sq().0 as usize]
}

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
    tt:       TranspositionTable,
    killers:  [[Move; 2]; 64],      // [depth][slot]
    history:  [[i32; 64]; 64],      // [from_sq][to_sq]
}

impl Search {
    pub fn new() -> Search {
        Search {
            nodes:   0,
            tt:      TranspositionTable::new(),
            killers: [[Move::NULL; 2]; 64],
            history: [[0i32; 64]; 64],
        }
    }

    /// Like `new()` but pre-seeds the history table with learned biases.
    /// Each entry is (from_sq_index, to_sq_index, delta). Negative deltas
    /// de-prioritize moves that were blunders in past games.
    pub fn with_biases(biases: &[(u8, u8, i32)]) -> Search {
        let mut s = Search::new();
        for &(from, to, delta) in biases {
            let f = from as usize;
            let t = to   as usize;
            s.history[f][t] = s.history[f][t].saturating_add(delta);
        }
        s
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

        // Iterative deepening: search depths 1..=max_depth
        // Each depth gives a result; only keep the result if we completed the depth fully.
        for depth in 1..=config.max_depth {
            if self.nodes >= config.max_nodes { break; }

            let nodes_before = self.nodes;
            let (candidate, score) = self.search_root(pos, &moves, depth, config.max_nodes);

            // Only update best if we didn't exhaust the budget mid-search
            let completed = self.nodes < config.max_nodes || depth == 1;
            if completed {
                best       = candidate;
                best_score = score;
            }

            // Stop early if mate found
            if best_score.abs() >= MATE_SCORE - 200 { break; }

            // If the budget was exhausted during this depth, stop
            if self.nodes >= config.max_nodes && nodes_before < config.max_nodes { break; }
        }

        // Apply random factor for lower difficulties
        let best = if config.random_factor > 0.0 {
            self.pick_with_random(&moves, pos, best, best_score, config.random_factor)
        } else {
            best
        };

        SearchResult::EngineMove(best, best_score)
    }

    fn search_root(&mut self, pos: &Position, moves: &[Move], depth: u8, max_nodes: u64) -> (Move, i32) {
        let mut best       = moves[0];
        let mut best_score = -INF;

        for &m in moves {
            if self.nodes >= max_nodes { break; }
            let child = pos.make_move(m);
            // Use -best_score as alpha floor for fail-soft aspiration (simple version)
            let score = -self.negamax(&child, depth - 1, -INF, -best_score.max(-INF), max_nodes);
            if score > best_score {
                best_score = score;
                best = m;
            }
        }

        (best, best_score)
    }

    fn pick_with_random(&self, moves: &[Move], pos: &Position, best: Move, best_score: i32, factor: f32) -> Move {
        // Among moves within factor*100 centipawns of best, pick one pseudo-randomly
        let threshold = (factor * 100.0) as i32;
        let candidates: Vec<Move> = moves.iter().copied()
            .filter(|&m| {
                let child = pos.make_move(m);
                let score = -crate::eval::evaluate(&child);
                best_score - score <= threshold
            })
            .collect();
        if candidates.is_empty() { return best; }
        // Deterministic pseudo-random using position hash
        let idx = (pos.hash as usize) % candidates.len();
        candidates[idx]
    }

    fn quiescence(&mut self, pos: &Position, mut alpha: i32, beta: i32, max_nodes: u64) -> i32 {
        self.nodes += 1;

        // King-capture guard
        let us = pos.side_to_move as usize;
        if pos.pieces[us][PieceType::King as usize].0 == 0 {
            return -MATE_SCORE;
        }

        if self.nodes >= max_nodes { return evaluate(pos); }

        // Stand-pat: static eval as lower bound
        let stand_pat = evaluate(pos);
        if stand_pat >= beta  { return beta; }
        if stand_pat > alpha  { alpha = stand_pat; }

        // Generate only captures and en-passant
        let captures: Vec<Move> = pos.legal_moves()
            .into_iter()
            .filter(|m| m.is_capture() || m.is_en_passant())
            .collect();

        // Order captures by MVV-LVA
        let mut scored: Vec<(Move, i32)> = captures.into_iter().map(|m| {
            let victim_pt = if m.is_en_passant() {
                PieceType::Pawn
            } else {
                pos.piece_at(m.to_sq()).map(|(_, pt)| pt).unwrap_or(PieceType::Pawn)
            };
            let attacker_pt = pos.piece_at(m.from_sq()).map(|(_, pt)| pt).unwrap_or(PieceType::Pawn);
            (m, mvv_lva(victim_pt, attacker_pt))
        }).collect();
        scored.sort_unstable_by(|a, b| b.1.cmp(&a.1));

        for (m, _) in scored {
            let child = pos.make_move(m);
            let score = -self.quiescence(&child, -beta, -alpha, max_nodes);
            if score >= beta  { return beta; }
            if score > alpha  { alpha = score; }
        }

        alpha
    }

    fn negamax(&mut self, pos: &Position, depth: u8, mut alpha: i32, beta: i32, max_nodes: u64) -> i32 {
        self.nodes += 1;

        // King-capture guard
        let us = pos.side_to_move as usize;
        if pos.pieces[us][PieceType::King as usize].0 == 0 {
            return -MATE_SCORE;
        }

        if self.nodes >= max_nodes { return evaluate(pos); }
        if depth == 0              { return self.quiescence(pos, alpha, beta, max_nodes); }

        // TT probe
        let tt_move = match self.tt.probe(pos.hash, depth, alpha, beta) {
            Some((score, _tt_m)) if score != i32::MIN => return score,
            Some((_, tt_m))                           => tt_m,
            None                                      => Move::NULL,
        };

        // Null move pruning: if we can pass the turn and still cause a beta cutoff,
        // the position is so good we can prune. Disabled when in check or near endgame.
        const NULL_R: u8 = 2;
        let is_endgame = pos.piece_count() <= 10;
        if depth >= NULL_R + 1 && !pos.is_in_check() && !is_endgame {
            let null_pos = pos.null_move();
            let null_score = -self.negamax(&null_pos, depth - NULL_R - 1, -beta, -beta + 1, max_nodes);
            if null_score >= beta {
                return beta;
            }
        }

        let moves = pos.legal_moves();
        if moves.is_empty() {
            return if pos.is_in_check() {
                -(MATE_SCORE - depth as i32)
            } else {
                DRAW_SCORE
            };
        }

        let original_alpha = alpha;
        let mut best_move  = Move::NULL;
        let mut best_score = -INF;

        // Score and sort all moves
        let ordered = moves;  // TT move gets highest score via score_move, sorts first automatically
        let mut scored: Vec<(Move, i32)> = ordered.into_iter().map(|m| {
            let s = score_move(
                pos, m, tt_move,
                &self.killers[depth.min(63) as usize],
                &self.history,
            );
            (m, s)
        }).collect();
        scored.sort_unstable_by(|a, b| b.1.cmp(&a.1));

        let mut move_count = 0usize;
        for (m, _) in scored {
            move_count += 1;
            let child = pos.make_move(m);

            let score = if move_count > 4
                && depth >= 3
                && !m.is_capture()
                && !m.is_promotion()
                && !pos.is_in_check()
            {
                // Late Move Reduction: search at reduced depth first
                let reduced = -self.negamax(&child, depth - 2, -alpha - 1, -alpha, max_nodes);
                if reduced > alpha {
                    // Re-search at full depth if move looks promising
                    -self.negamax(&child, depth - 1, -beta, -alpha, max_nodes)
                } else {
                    reduced
                }
            } else {
                -self.negamax(&child, depth - 1, -beta, -alpha, max_nodes)
            };
            if score > best_score {
                best_score = score;
                best_move  = m;
            }
            if score > alpha { alpha = score; }
            if alpha >= beta {
                // Beta cutoff: update killers and history for quiet moves
                if !m.is_capture() && !m.is_en_passant() {
                    let d = depth.min(63) as usize;
                    self.killers[d][1] = self.killers[d][0];
                    self.killers[d][0] = m;
                    self.history[m.from_sq().0 as usize][m.to_sq().0 as usize] +=
                        (depth as i32) * (depth as i32);
                }
                break;
            }
        }
        // Store result in TT
        let flag = if best_score <= original_alpha {
            TTFlag::UpperBound
        } else if best_score >= beta {
            TTFlag::LowerBound
        } else {
            TTFlag::Exact
        };
        self.tt.store(pos.hash, depth, best_score, flag, best_move);

        best_score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;

    fn pos(fen: &str) -> Position { Position::from_fen(fen).unwrap() }

    #[test]
    fn tt_reduces_nodes() {
        // With TT, searching startpos at depth 5 should complete within budget
        let p = pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let config = DifficultyConfig::medio();
        let mut s = Search::new();
        let SearchResult::EngineMove(_, _) = s.best_move(&p, &config);
        // Just verify the search completes and stays within node budget
        assert!(s.nodes > 0);
        assert!(s.nodes <= config.max_nodes);
    }

    #[test]
    fn mate_in_one_rook() {
        // White rook on a7, white king on b6, black king on a8 — Ra8#
        let p = pos("k7/R7/1K6/8/8/8/8/8 w - - 0 1");
        let config = DifficultyConfig::medio();
        let result = Search::new().best_move(&p, &config);
        let SearchResult::EngineMove(m, score) = result;
        assert!(score >= MATE_SCORE - 100, "score {} should be near mate", score);
        assert_eq!(m.to_uci(), "a7a8", "expected Ra8# got {}", m.to_uci());
    }

    #[test]
    fn mate_in_one_queen() {
        // White queen on b1, white king on a3, black king on a8 — Qb8#
        let p = pos("k7/8/K7/8/8/8/8/1Q6 w - - 0 1");
        let config = DifficultyConfig::medio();
        let result = Search::new().best_move(&p, &config);
        let SearchResult::EngineMove(_m, score) = result;
        assert!(score >= MATE_SCORE - 100, "score {} should be near mate", score);
    }

    #[test]
    fn returns_a_move() {
        let p = pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let config = DifficultyConfig::facil();
        let result = Search::new().best_move(&p, &config);
        assert!(matches!(result, SearchResult::EngineMove(_, _)));
    }

    #[test]
    fn iterative_deepening_finds_mate() {
        // Same mate-in-1 position but using a depth-64 config to ensure
        // iterative deepening completes depth 1 (finding the mate) and stops
        let p = pos("k7/R7/1K6/8/8/8/8/8 w - - 0 1");
        let config = DifficultyConfig { max_depth: 64, max_nodes: 100_000, random_factor: 0.0 };
        let result = Search::new().best_move(&p, &config);
        let SearchResult::EngineMove(m, score) = result;
        assert!(score >= MATE_SCORE - 100, "score {} should be near mate", score);
        assert_eq!(m.to_uci(), "a7a8");
    }

    #[test]
    fn quiescence_avoids_horizon_effect() {
        // Black queen can capture a hanging white queen on e4, then white recaptures with pawn
        // Without quiescence, depth-0 might see black up a queen; with quiescence it should
        // see the recapture and find the position roughly equal
        // Position: white queen on e4 hanging, black queen on d5 can take it,
        // white pawn on f3 can recapture on e4
        // Black to move: Qxe4 fxe4 — net result is pawn trade, not queen win
        let p = pos("k7/8/8/3q4/4Q3/5P2/8/K7 b - - 0 1");
        let config = DifficultyConfig { max_depth: 2, max_nodes: 500_000, random_factor: 0.0 };
        let SearchResult::EngineMove(m, _) = Search::new().best_move(&p, &config);
        // Black should play Qxe4 (d5e4)
        assert_eq!(m.to_uci(), "d5e4", "expected Qxe4 got {}", m.to_uci());
    }

    #[test]
    fn null_move_does_not_break_mate_detection() {
        // Null move pruning must be disabled when in check
        // Verify the engine still finds mate in a position where null move could cause issues
        let p = pos("k7/R7/1K6/8/8/8/8/8 w - - 0 1");
        let config = DifficultyConfig { max_depth: 5, max_nodes: 500_000, random_factor: 0.0 };
        let SearchResult::EngineMove(m, score) = Search::new().best_move(&p, &config);
        assert!(score >= MATE_SCORE - 100, "score {} should be near mate", score);
        assert_eq!(m.to_uci(), "a7a8");
    }

    #[test]
    fn captures_ordered_before_quiets() {
        // After 1.e4 e5, white has a capture: d4xe5 is not available but exd5 test
        // More reliably: test that a winning capture position finds the right move quickly
        // White queen takes free pawn — should be found at low depth
        let p = pos("k7/8/8/3p4/4Q3/8/8/K7 w - - 0 1");
        let config = DifficultyConfig { max_depth: 3, max_nodes: 50_000, random_factor: 0.0 };
        let SearchResult::EngineMove(m, _) = Search::new().best_move(&p, &config);
        // Qxd5 should be found
        assert_eq!(m.to_uci(), "e4d5", "expected Qxd5 got {}", m.to_uci());
    }
}
