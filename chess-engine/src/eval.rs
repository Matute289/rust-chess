use crate::position::Position;
use crate::types::{Color, PieceType};

// ── Material values ───────────────────────────────────────────────────────────

pub const PAWN_VALUE:   i32 = 100;
pub const KNIGHT_VALUE: i32 = 320;
pub const BISHOP_VALUE: i32 = 330;
pub const ROOK_VALUE:   i32 = 500;
pub const QUEEN_VALUE:  i32 = 900;
pub const KING_VALUE:   i32 = 20_000;

pub fn piece_value(pt: PieceType) -> i32 {
    match pt {
        PieceType::Pawn   => PAWN_VALUE,
        PieceType::Knight => KNIGHT_VALUE,
        PieceType::Bishop => BISHOP_VALUE,
        PieceType::Rook   => ROOK_VALUE,
        PieceType::Queen  => QUEEN_VALUE,
        PieceType::King   => KING_VALUE,
    }
}

// ── Game phase ────────────────────────────────────────────────────────────────
// Phase weights: N=1, B=1, R=2, Q=4 (pawns and kings don't count)
// MAX_PHASE = 2 queens (8) + 4 rooks (8) + 4 bishops (4) + 4 knights (4) = 24
pub const MAX_PHASE: i32 = 24;

const PHASE_WEIGHT: [i32; 6] = [0, 1, 1, 2, 4, 0]; // [Pawn, Knight, Bishop, Rook, Queen, King]

/// Returns a value in [0, MAX_PHASE]: MAX_PHASE = opening, 0 = pure endgame.
pub fn game_phase(pos: &Position) -> i32 {
    let mut phase = 0i32;
    for c in 0..2usize {
        for pt in 0..6usize {
            let count = pos.pieces[c][pt].count() as i32;
            phase += count * PHASE_WEIGHT[pt];
        }
    }
    phase.min(MAX_PHASE)
}

// ── Piece-Square Tables ───────────────────────────────────────────────────────
// Indexed by sq: A1=0, H1=7, A2=8, ..., H8=63
// White: use sq directly. Black: use sq ^ 56 (flip rank).

const PST_PAWN_OP: [i32; 64] = [
//  A    B    C    D    E    F    G    H
    0,   0,   0,   0,   0,   0,   0,   0,  // rank 1 (impossible for pawns)
    5,  10,  10, -20, -20,  10,  10,   5,  // rank 2 (start)
    5,  -5, -10,   0,   0, -10,  -5,   5,  // rank 3
    0,   0,   0,  20,  20,   0,   0,   0,  // rank 4
    5,   5,  10,  25,  25,  10,   5,   5,  // rank 5
   10,  10,  20,  30,  30,  20,  10,  10,  // rank 6
   50,  50,  50,  50,  50,  50,  50,  50,  // rank 7
    0,   0,   0,   0,   0,   0,   0,   0,  // rank 8
];

const PST_PAWN_EG: [i32; 64] = [
    0,   0,   0,   0,   0,   0,   0,   0,
    0,   0,   0,   0,   0,   0,   0,   0,
    5,   5,   5,   5,   5,   5,   5,   5,
   10,  10,  10,  10,  10,  10,  10,  10,
   20,  20,  20,  20,  20,  20,  20,  20,
   30,  30,  30,  30,  30,  30,  30,  30,
   50,  50,  50,  50,  50,  50,  50,  50,
    0,   0,   0,   0,   0,   0,   0,   0,
];

const PST_KNIGHT: [i32; 64] = [
  -50, -40, -30, -30, -30, -30, -40, -50,
  -40, -20,   0,   0,   0,   0, -20, -40,
  -30,   0,  10,  15,  15,  10,   0, -30,
  -30,   5,  15,  20,  20,  15,   5, -30,
  -30,   0,  15,  20,  20,  15,   0, -30,
  -30,   5,  10,  15,  15,  10,   5, -30,
  -40, -20,   0,   5,   5,   0, -20, -40,
  -50, -40, -30, -30, -30, -30, -40, -50,
];

const PST_BISHOP: [i32; 64] = [
  -20, -10, -10, -10, -10, -10, -10, -20,
  -10,   0,   0,   0,   0,   0,   0, -10,
  -10,   0,   5,  10,  10,   5,   0, -10,
  -10,   5,   5,  10,  10,   5,   5, -10,
  -10,   0,  10,  10,  10,  10,   0, -10,
  -10,  10,  10,  10,  10,  10,  10, -10,
  -10,   5,   0,   0,   0,   0,   5, -10,
  -20, -10, -10, -10, -10, -10, -10, -20,
];

const PST_ROOK: [i32; 64] = [
    0,  0,  0,  5,  5,  0,  0,  0,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
    5, 10, 10, 10, 10, 10, 10,  5,
    0,  0,  0,  0,  0,  0,  0,  0,
];

const PST_QUEEN: [i32; 64] = [
  -20, -10, -10, -5, -5, -10, -10, -20,
  -10,   0,   0,  0,  0,   0,   0, -10,
  -10,   0,   5,  5,  5,   5,   0, -10,
   -5,   0,   5,  5,  5,   5,   0,  -5,
    0,   0,   5,  5,  5,   5,   0,  -5,
  -10,   5,   5,  5,  5,   5,   0, -10,
  -10,   0,   5,  0,  0,   0,   0, -10,
  -20, -10, -10, -5, -5, -10, -10, -20,
];

const PST_KING_OP: [i32; 64] = [
   20,  30,  10,   0,   0,  10,  30,  20,  // rank 1: g1/b1 good (castled)
   20,  20,   0,   0,   0,   0,  20,  20,
  -10, -20, -20, -20, -20, -20, -20, -10,
  -20, -30, -30, -40, -40, -30, -30, -20,
  -30, -40, -40, -50, -50, -40, -40, -30,
  -30, -40, -40, -50, -50, -40, -40, -30,
  -30, -40, -40, -50, -50, -40, -40, -30,
  -30, -40, -40, -50, -50, -40, -40, -30,
];

const PST_KING_EG: [i32; 64] = [
  -50, -30, -30, -30, -30, -30, -30, -50,
  -30, -30,   0,   0,   0,   0, -30, -30,
  -30, -10,  20,  30,  30,  20, -10, -30,
  -30, -10,  30,  40,  40,  30, -10, -30,
  -30, -10,  30,  40,  40,  30, -10, -30,
  -30, -10,  20,  30,  30,  20, -10, -30,
  -30, -20,  -10,   0,   0, -10, -20, -30,
  -50, -40, -30, -20, -20, -30, -40, -50,
];

fn pst_score(pt: PieceType, idx: usize, phase: i32) -> i32 {
    match pt {
        PieceType::Pawn => {
            let op = PST_PAWN_OP[idx];
            let eg = PST_PAWN_EG[idx];
            (op * phase + eg * (MAX_PHASE - phase)) / MAX_PHASE
        }
        PieceType::Knight => PST_KNIGHT[idx],
        PieceType::Bishop => PST_BISHOP[idx],
        PieceType::Rook   => PST_ROOK[idx],
        PieceType::Queen  => PST_QUEEN[idx],
        PieceType::King => {
            let op = PST_KING_OP[idx];
            let eg = PST_KING_EG[idx];
            (op * phase + eg * (MAX_PHASE - phase)) / MAX_PHASE
        }
    }
}

// ── Main eval ─────────────────────────────────────────────────────────────────

/// Returns centipawns from side-to-move perspective (positive = good for mover).
pub fn evaluate(pos: &Position) -> i32 {
    let us    = pos.side_to_move;
    let them  = us.flip();
    let phase = game_phase(pos);

    let mut score = 0i32;

    for pt in [PieceType::Pawn, PieceType::Knight, PieceType::Bishop,
               PieceType::Rook, PieceType::Queen, PieceType::King] {
        // Material
        let our_count   = pos.pieces[us as usize][pt as usize].count() as i32;
        let their_count = pos.pieces[them as usize][pt as usize].count() as i32;
        score += (our_count - their_count) * piece_value(pt);

        // PST — White perspective: use sq directly; Black: flip rank with ^ 56
        for sq in pos.pieces[us as usize][pt as usize].squares() {
            let idx = if us == Color::White { sq.0 as usize } else { (sq.0 ^ 56) as usize };
            score += pst_score(pt, idx, phase);
        }
        for sq in pos.pieces[them as usize][pt as usize].squares() {
            let idx = if them == Color::White { sq.0 as usize } else { (sq.0 ^ 56) as usize };
            score -= pst_score(pt, idx, phase);
        }
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;

    fn pos(fen: &str) -> Position { Position::from_fen(fen).unwrap() }

    #[test]
    fn startpos_is_balanced() {
        let score = evaluate(&pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"));
        assert!(score.abs() < 50, "startpos score {} too far from 0", score);
    }

    #[test]
    fn white_up_a_queen() {
        // White has an extra queen on a3
        let score = evaluate(&pos("rnbqkbnr/pppppppp/8/8/8/Q7/PPPPPPPP/RNBQKBNR w KQkq - 0 1"));
        assert!(score > 800, "extra queen score {} too low", score);
    }

    #[test]
    fn black_up_a_rook() {
        // White missing the a1 rook — score from white's perspective should be around -500
        let score = evaluate(&pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/1NBQKBNR w Kkq - 0 1"));
        assert!(score < -400, "missing rook score {} too high", score);
    }

    #[test]
    fn game_phase_full_material() {
        let phase = game_phase(&pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"));
        assert_eq!(phase, MAX_PHASE);
    }

    #[test]
    fn game_phase_kings_only() {
        let phase = game_phase(&pos("k7/8/8/8/8/8/8/K7 w - - 0 1"));
        assert_eq!(phase, 0);
    }

    #[test]
    fn knight_prefers_center() {
        // Knight on e4 (center) vs b1 (rim) — same material, only PST differs
        let center = evaluate(&pos("8/8/8/8/4N3/8/8/K1k5 w - - 0 1"));
        let corner  = evaluate(&pos("8/8/8/8/8/8/8/KNk5 w - - 0 1"));
        assert!(center > corner, "knight center {} should beat corner {}", center, corner);
    }

    #[test]
    fn king_prefers_corner_in_opening() {
        // In opening phase (full material), king on g1 (castled) beats king on e1 (center).
        // Both positions have identical material — only white king square differs.
        // castled: white king g1, center: white king e1; all other pieces identical.
        let castled = evaluate(&pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQ1BK1 w kq - 0 1"));
        let center  = evaluate(&pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKB2 w kq - 0 1"));
        assert!(castled > center, "castled king {} should beat center king {}", castled, center);
    }
}
