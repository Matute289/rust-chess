use crate::position::Position;
use crate::types::PieceType;

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

// ── Main eval ─────────────────────────────────────────────────────────────────

/// Returns centipawns from side-to-move perspective (positive = good for mover).
pub fn evaluate(pos: &Position) -> i32 {
    let us   = pos.side_to_move;
    let them = us.flip();

    let mut score = 0i32;
    for pt in [PieceType::Pawn, PieceType::Knight, PieceType::Bishop,
               PieceType::Rook, PieceType::Queen, PieceType::King] {
        let diff = pos.pieces[us as usize][pt as usize].count() as i32
                 - pos.pieces[them as usize][pt as usize].count() as i32;
        score += diff * piece_value(pt);
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
}
