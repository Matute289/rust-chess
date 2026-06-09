use crate::bitboard::Bitboard;
use crate::position::Position;
use crate::types::{Color, PieceType, Square};

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
//  A    B    C    D    E    F    G    H
  -50, -40, -20, -30, -30, -20, -40, -50,  // rank 1 (indices 0-7)
  -40, -20,   0,   5,   5,   0, -20, -40,  // rank 2
  -30,   5,  10,  15,  15,  10,   5, -30,  // rank 3
  -30,   0,  15,  20,  20,  15,   0, -30,  // rank 4
  -30,   5,  15,  20,  20,  15,   5, -30,  // rank 5
  -30,   0,  10,  15,  15,  10,   0, -30,  // rank 6
  -40, -20,   0,   0,   0,   0, -20, -40,  // rank 7
  -50, -40, -30, -30, -30, -30, -40, -50,  // rank 8 (indices 56-63)
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

// ── Pawn structure ────────────────────────────────────────────────────────────

const FILE_MASKS: [u64; 8] = [
    0x0101010101010101,  // file A
    0x0202020202020202,  // file B
    0x0404040404040404,  // file C
    0x0808080808080808,  // file D
    0x1010101010101010,  // file E
    0x2020202020202020,  // file F
    0x4040404040404040,  // file G
    0x8080808080808080,  // file H
];

fn pawn_structure_score(pos: &Position, us: Color) -> i32 {
    let them = us.flip();
    let our_pawns   = pos.pieces[us as usize][PieceType::Pawn as usize];
    let their_pawns = pos.pieces[them as usize][PieceType::Pawn as usize];
    let mut score = 0i32;

    for sq in our_pawns.squares() {
        let file = sq.file() as usize;
        let rank = sq.rank();

        // Doubled pawn penalty: more than one friendly pawn on same file
        let pawns_on_file = (our_pawns & Bitboard(FILE_MASKS[file])).count();
        if pawns_on_file > 1 {
            score -= 20;
        }

        // Isolated pawn penalty: no friendly pawns on adjacent files
        let left  = if file > 0 { Bitboard(FILE_MASKS[file - 1]) } else { Bitboard::EMPTY };
        let right = if file < 7 { Bitboard(FILE_MASKS[file + 1]) } else { Bitboard::EMPTY };
        let adj_files = left | right;
        if (our_pawns & adj_files).is_empty() {
            score -= 15;
        }

        // Passed pawn bonus: no enemy pawns on same or adjacent files ahead of us
        let same_and_adj = Bitboard(FILE_MASKS[file]) | adj_files;
        let front_mask = if us == Color::White {
            // All squares on ranks above our rank
            Bitboard(same_and_adj.0 & !((1u64 << ((rank + 1) * 8)) - 1))
        } else {
            // All squares on ranks below our rank (rank 0..rank-1)
            if rank == 0 { Bitboard::EMPTY } else {
                Bitboard(same_and_adj.0 & ((1u64 << (rank * 8)) - 1))
            }
        };

        if (their_pawns & front_mask).is_empty() {
            // Passed pawn: bonus grows with advancement
            let advance = if us == Color::White { rank } else { 7 - rank };
            score += 10 + advance as i32 * 5;
        }
    }

    score
}

// ── Mobility ──────────────────────────────────────────────────────────────────

fn mobility_score(pos: &Position, us: Color) -> i32 {
    use crate::tables::Tables;
    let t   = Tables::get();
    let occ = pos.occupied();
    let mut attacks = Bitboard::EMPTY;

    for sq in pos.pieces[us as usize][PieceType::Knight as usize].squares() {
        attacks |= t.knight_attacks[sq.0 as usize];
    }
    for sq in pos.pieces[us as usize][PieceType::Bishop as usize].squares() {
        attacks |= t.bishop_attacks(sq, occ);
    }
    for sq in pos.pieces[us as usize][PieceType::Rook as usize].squares() {
        attacks |= t.rook_attacks(sq, occ);
    }
    for sq in pos.pieces[us as usize][PieceType::Queen as usize].squares() {
        attacks |= t.queen_attacks(sq, occ);
    }

    attacks.count() as i32
}

// ── King safety ───────────────────────────────────────────────────────────────

fn king_safety_score(pos: &Position, us: Color, phase: i32) -> i32 {
    // Only meaningful in opening/middlegame
    if phase < MAX_PHASE / 3 { return 0; }

    let king_sq   = pos.king_sq(us);
    let king_file = king_sq.file();
    let pawns     = pos.pieces[us as usize][PieceType::Pawn as usize];
    let shield_rank = if us == Color::White { 1u8 } else { 6u8 };
    let mut score = 0i32;

    // Pawn shield: count pawns on the 3-file zone in front of the king
    let f_start = king_file.saturating_sub(1);
    let f_end   = (king_file + 1).min(7);
    for f in f_start..=f_end {
        if pawns.get(Square::from_rank_file(shield_rank, f)) { score += 10; }
    }

    // Center king penalty in opening
    if king_file >= 2 && king_file <= 5 {
        score -= 20 * phase / MAX_PHASE;
    }

    score
}

// ── Center control ────────────────────────────────────────────────────────────

fn center_control_score(pos: &Position, us: Color) -> i32 {
    use crate::movegen::MoveGen;
    // Central squares: d4=27, e4=28, d5=35, e5=36
    const CENTER: [Square; 4] = [Square(27), Square(28), Square(35), Square(36)];
    let mut score = 0i32;
    for &sq in &CENTER {
        if MoveGen::is_attacked(pos, sq, us)          { score += 5; }
        if MoveGen::is_attacked(pos, sq, us.flip())   { score -= 5; }
    }
    score
}

// ── Main eval ─────────────────────────────────────────────────────────────────

/// Returns centipawns from side-to-move perspective (positive = good for mover).
pub fn evaluate(pos: &Position) -> i32 {
    let us    = pos.side_to_move;
    let them  = us.flip();
    let phase = game_phase(pos);

    let mut score = 0i32;

    // Material + PST
    for pt in [PieceType::Pawn, PieceType::Knight, PieceType::Bishop,
               PieceType::Rook, PieceType::Queen, PieceType::King] {
        let our_count   = pos.pieces[us as usize][pt as usize].count() as i32;
        let their_count = pos.pieces[them as usize][pt as usize].count() as i32;
        score += (our_count - their_count) * piece_value(pt);

        for sq in pos.pieces[us as usize][pt as usize].squares() {
            let idx = if us == Color::White { sq.0 as usize } else { (sq.0 ^ 56) as usize };
            score += pst_score(pt, idx, phase);
        }
        for sq in pos.pieces[them as usize][pt as usize].squares() {
            let idx = if them == Color::White { sq.0 as usize } else { (sq.0 ^ 56) as usize };
            score -= pst_score(pt, idx, phase);
        }
    }

    // Pawn structure
    score += pawn_structure_score(pos, us);
    score -= pawn_structure_score(pos, them);

    // Mobility (2cp per attacked square advantage)
    score += (mobility_score(pos, us) - mobility_score(pos, them)) * 2;

    // King safety
    score += king_safety_score(pos, us, phase);
    score -= king_safety_score(pos, them, phase);

    // Center control
    score += center_control_score(pos, us);

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

    #[test]
    fn doubled_pawns_penalty() {
        // White has doubled pawns on e-file (e2 + e3), black has normal spread
        let doubled = evaluate(&pos("k7/8/8/8/8/4P3/4P3/K7 w - - 0 1"));
        let normal  = evaluate(&pos("k7/8/8/8/8/3P4/4P3/K7 w - - 0 1"));
        assert!(doubled < normal, "doubled {} should be worse than normal {}", doubled, normal);
    }

    #[test]
    fn passed_pawn_bonus() {
        // White pawn on e5 with no black pawns blocking its path to e8
        let passed  = evaluate(&pos("k7/8/8/4P3/8/8/8/K7 w - - 0 1"));
        // Same but black pawn on e7 blocks and contests the path
        let blocked = evaluate(&pos("k7/4p3/8/4P3/8/8/8/K7 w - - 0 1"));
        assert!(passed > blocked, "passed {} should beat blocked {}", passed, blocked);
    }
}
