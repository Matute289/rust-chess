use std::sync::OnceLock;
use crate::bitboard::Bitboard;
use crate::types::Square;

pub struct MagicEntry {
    pub mask:    Bitboard,
    pub magic:   u64,
    pub shift:   u32,
    pub attacks: Vec<Bitboard>,
}

pub struct Tables {
    pub knight_attacks: [Bitboard; 64],
    pub king_attacks:   [Bitboard; 64],
    pub pawn_attacks:   [[Bitboard; 64]; 2],
    pub bishop_magics:  Box<[MagicEntry; 64]>,
    pub rook_magics:    Box<[MagicEntry; 64]>,
}

static TABLES: OnceLock<Tables> = OnceLock::new();

impl Tables {
    pub fn get() -> &'static Tables {
        TABLES.get_or_init(Tables::init)
    }

    fn init() -> Tables {
        Tables {
            knight_attacks: compute_knight_attacks(),
            king_attacks:   compute_king_attacks(),
            pawn_attacks:   compute_pawn_attacks(),
            bishop_magics:  generate_magic_tables(false),
            rook_magics:    generate_magic_tables(true),
        }
    }

    pub fn bishop_attacks(&self, sq: Square, occupancy: Bitboard) -> Bitboard {
        let entry = &self.bishop_magics[sq.0 as usize];
        let idx = ((occupancy & entry.mask).0.wrapping_mul(entry.magic) >> entry.shift) as usize;
        entry.attacks[idx]
    }

    pub fn rook_attacks(&self, sq: Square, occupancy: Bitboard) -> Bitboard {
        let entry = &self.rook_magics[sq.0 as usize];
        let idx = ((occupancy & entry.mask).0.wrapping_mul(entry.magic) >> entry.shift) as usize;
        entry.attacks[idx]
    }

    pub fn queen_attacks(&self, sq: Square, occupancy: Bitboard) -> Bitboard {
        self.bishop_attacks(sq, occupancy) | self.rook_attacks(sq, occupancy)
    }
}

fn compute_knight_attacks() -> [Bitboard; 64] {
    const NOT_AB: u64 = !0x0303030303030303u64;
    const NOT_GH: u64 = !0xC0C0C0C0C0C0C0C0u64;
    const NOT_A:  u64 = !0x0101010101010101u64;
    const NOT_H:  u64 = !0x8080808080808080u64;
    let mut attacks = [Bitboard::EMPTY; 64];
    for sq in 0u8..64 {
        let bb = 1u64 << sq;
        attacks[sq as usize] = Bitboard(
              ((bb & NOT_AB) >> 10) | ((bb & NOT_AB) << 6)
            | ((bb & NOT_A)  >> 17) | ((bb & NOT_A)  << 15)
            | ((bb & NOT_H)  >> 15) | ((bb & NOT_H)  << 17)
            | ((bb & NOT_GH) >>  6) | ((bb & NOT_GH) << 10)
        );
    }
    attacks
}

fn compute_king_attacks() -> [Bitboard; 64] {
    const NOT_A: u64 = !0x0101010101010101u64;
    const NOT_H: u64 = !0x8080808080808080u64;
    let mut attacks = [Bitboard::EMPTY; 64];
    for sq in 0u8..64 {
        let bb = 1u64 << sq;
        attacks[sq as usize] = Bitboard(
              (bb >> 8)
            | (bb << 8)
            | ((bb & NOT_H) << 1)
            | ((bb & NOT_A) >> 1)
            | ((bb & NOT_H) << 9)
            | ((bb & NOT_A) << 7)
            | ((bb & NOT_H) >> 7)
            | ((bb & NOT_A) >> 9)
        );
    }
    attacks
}

fn compute_pawn_attacks() -> [[Bitboard; 64]; 2] {
    const NOT_A: u64 = !0x0101010101010101u64;
    const NOT_H: u64 = !0x8080808080808080u64;
    let mut attacks = [[Bitboard::EMPTY; 64]; 2];
    for sq in 0u8..64 {
        let bb = 1u64 << sq;
        // White attacks NE (<<9, mask FILE_H) and NW (<<7, mask FILE_A)
        attacks[0][sq as usize] = Bitboard(
            ((bb & NOT_H) << 9) | ((bb & NOT_A) << 7)
        );
        // Black attacks SE (>>7, mask FILE_H) and SW (>>9, mask FILE_A)
        attacks[1][sq as usize] = Bitboard(
            ((bb & NOT_H) >> 7) | ((bb & NOT_A) >> 9)
        );
    }
    attacks
}

// ── Classical ray-based attacks (used during magic table generation only) ────

fn rook_mask(sq: u8) -> Bitboard {
    // Relevant occupancy: ranks and files through sq, excluding edges and sq itself
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let mut mask = 0u64;
    for r in 1i8..7 { if r != rank { mask |= 1u64 << (r * 8 + file); } }
    for f in 1i8..7 { if f != file { mask |= 1u64 << (rank * 8 + f); } }
    Bitboard(mask)
}

fn bishop_mask(sq: u8) -> Bitboard {
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let mut mask = 0u64;
    for (dr, df) in [(1i8,1i8),(1,-1),(-1,1),(-1,-1)] {
        let (mut r, mut f) = (rank + dr, file + df);
        while r > 0 && r < 7 && f > 0 && f < 7 {
            mask |= 1u64 << (r * 8 + f);
            r += dr; f += df;
        }
    }
    Bitboard(mask)
}

fn rook_attacks_classical(sq: u8, blockers: Bitboard) -> Bitboard {
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let mut attacks = 0u64;
    for (dr, df) in [(1i8,0i8),(-1,0),(0,1),(0,-1)] {
        let (mut r, mut f) = (rank + dr, file + df);
        while r >= 0 && r < 8 && f >= 0 && f < 8 {
            let s = (r * 8 + f) as u8;
            attacks |= 1u64 << s;
            if blockers.get(Square(s)) { break; }
            r += dr; f += df;
        }
    }
    Bitboard(attacks)
}

fn bishop_attacks_classical(sq: u8, blockers: Bitboard) -> Bitboard {
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let mut attacks = 0u64;
    for (dr, df) in [(1i8,1i8),(1,-1),(-1,1),(-1,-1)] {
        let (mut r, mut f) = (rank + dr, file + df);
        while r >= 0 && r < 8 && f >= 0 && f < 8 {
            let s = (r * 8 + f) as u8;
            attacks |= 1u64 << s;
            if blockers.get(Square(s)) { break; }
            r += dr; f += df;
        }
    }
    Bitboard(attacks)
}

fn xorshift(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

fn generate_magic_tables(is_rook: bool) -> Box<[MagicEntry; 64]> {
    let mut seed = 0x123456789ABCDEFu64;
    let entries: Vec<MagicEntry> = (0..64u8).map(|sq| {
        let mask = if is_rook { rook_mask(sq) } else { bishop_mask(sq) };
        let bits = mask.count();
        let n = 1usize << bits;

        // Enumerate all subsets of mask using Carry-Rippler trick
        let mut occ = vec![Bitboard::EMPTY; n];
        let mut att = vec![Bitboard::EMPTY; n];
        let mut subset = 0u64;
        for i in 0..n {
            occ[i] = Bitboard(subset);
            att[i] = if is_rook {
                rook_attacks_classical(sq, Bitboard(subset))
            } else {
                bishop_attacks_classical(sq, Bitboard(subset))
            };
            // Carry-Rippler: next subset of mask
            subset = subset.wrapping_sub(mask.0) & mask.0;
        }

        // Brute-force magic search
        loop {
            // Sparse random: AND three random values to get a number with few set bits
            let magic = xorshift(&mut seed) & xorshift(&mut seed) & xorshift(&mut seed);
            // Quick filter: upper byte after multiply should be populated
            if (mask.0.wrapping_mul(magic) >> 56).count_ones() < 6 { continue; }

            let shift = 64 - bits;
            let mut used = vec![Bitboard::EMPTY; n];
            let mut ok = true;

            for i in 0..n {
                let idx = (occ[i].0.wrapping_mul(magic) >> shift) as usize;
                if used[idx] == Bitboard::EMPTY {
                    used[idx] = att[i];
                } else if used[idx] != att[i] {
                    ok = false;
                    break;
                }
            }

            if ok {
                return MagicEntry { mask, magic, shift, attacks: used };
            }
        }
    }).collect();

    entries
        .into_boxed_slice()
        .try_into()
        .unwrap_or_else(|_| panic!("expected 64 magic entries"))
}

fn stub_magic_tables() -> Box<[MagicEntry; 64]> {
    let boxed: Box<[MagicEntry; 64]> = (0..64usize)
        .map(|_| MagicEntry { mask: Bitboard::EMPTY, magic: 0, shift: 0, attacks: vec![] })
        .collect::<Vec<_>>()
        .into_boxed_slice()
        .try_into()
        .unwrap_or_else(|_| panic!("expected 64"));
    boxed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Color, Square};

    #[test]
    fn knight_e4_attacks() {
        let t = Tables::get();
        // e4 = square 28 (rank 3, file 4)
        // from e4 a knight reaches: d2(11), f2(13), c3(18), g3(22), c5(34), g5(38), d6(43), f6(45)
        let attacks = t.knight_attacks[28];
        assert_eq!(attacks.count(), 8);
        assert!(attacks.get(Square(11))); // d2
        assert!(attacks.get(Square(13))); // f2
        assert!(attacks.get(Square(18))); // c3
        assert!(attacks.get(Square(22))); // g3
        assert!(attacks.get(Square(34))); // c5
        assert!(attacks.get(Square(38))); // g5
        assert!(attacks.get(Square(43))); // d6
        assert!(attacks.get(Square(45))); // f6
    }

    #[test]
    fn knight_a1_attacks() {
        let t = Tables::get();
        let attacks = t.knight_attacks[0]; // a1
        assert_eq!(attacks.count(), 2);
        assert!(attacks.get(Square(10))); // b3
        assert!(attacks.get(Square(17))); // c2
    }

    #[test]
    fn knight_h8_attacks() {
        let t = Tables::get();
        let attacks = t.knight_attacks[63]; // h8
        assert_eq!(attacks.count(), 2);
        assert!(attacks.get(Square(46))); // g6
        assert!(attacks.get(Square(53))); // f7
    }

    #[test]
    fn king_e1_attacks() {
        let t = Tables::get();
        let attacks = t.king_attacks[4]; // e1
        // d1(3), f1(5), d2(11), e2(12), f2(13) — 5 squares
        assert_eq!(attacks.count(), 5);
        assert!(attacks.get(Square(3)));  // d1
        assert!(attacks.get(Square(5)));  // f1
        assert!(attacks.get(Square(11))); // d2
        assert!(attacks.get(Square(12))); // e2
        assert!(attacks.get(Square(13))); // f2
    }

    #[test]
    fn king_a1_attacks() {
        let t = Tables::get();
        let attacks = t.king_attacks[0]; // a1
        assert_eq!(attacks.count(), 3);
    }

    #[test]
    fn white_pawn_e4_attacks() {
        let t = Tables::get();
        let attacks = t.pawn_attacks[Color::White as usize][28]; // e4
        assert_eq!(attacks.count(), 2);
        assert!(attacks.get(Square(35))); // d5
        assert!(attacks.get(Square(37))); // f5
    }

    #[test]
    fn black_pawn_e5_attacks() {
        let t = Tables::get();
        let attacks = t.pawn_attacks[Color::Black as usize][36]; // e5
        assert_eq!(attacks.count(), 2);
        assert!(attacks.get(Square(27))); // d4
        assert!(attacks.get(Square(29))); // f4
    }

    #[test]
    fn white_pawn_a2_attacks() {
        let t = Tables::get();
        // a-file pawn: only one attack square (b3)
        let attacks = t.pawn_attacks[Color::White as usize][8]; // a2
        assert_eq!(attacks.count(), 1);
        assert!(attacks.get(Square(17))); // b3
    }

    #[test]
    fn rook_a1_open_board() {
        let t = Tables::get();
        // Rook on a1 (sq 0), empty board: full rank 1 (minus a1) + full file a (minus a1)
        // = 7 (rank 1: b1-h1) + 7 (file a: a2-a8) = 14 squares
        let attacks = t.rook_attacks(Square(0), Bitboard::EMPTY);
        assert_eq!(attacks.count(), 14);
        assert!(attacks.get(Square(7)));  // h1
        assert!(attacks.get(Square(56))); // a8
        assert!(!attacks.get(Square(0))); // not a1 itself
    }

    #[test]
    fn rook_a1_with_blocker_on_c1_and_a3() {
        let t = Tables::get();
        let occ = Bitboard::EMPTY.set(Square(2)).set(Square(16)); // c1 and a3
        let attacks = t.rook_attacks(Square(0), occ);
        // East: b1(1), c1(2) — stops at c1 (included as capture target)
        assert!(attacks.get(Square(1)));  // b1
        assert!(attacks.get(Square(2)));  // c1 (capture)
        assert!(!attacks.get(Square(3))); // d1 — behind blocker
        // North: a2(8), a3(16) — stops at a3
        assert!(attacks.get(Square(8)));  // a2
        assert!(attacks.get(Square(16))); // a3 (capture)
        assert!(!attacks.get(Square(24))); // a4 — behind blocker
    }

    #[test]
    fn bishop_d4_open_board() {
        let t = Tables::get();
        // Bishop on d4 (sq 27): 13 diagonal squares on open board
        let attacks = t.bishop_attacks(Square(27), Bitboard::EMPTY);
        assert_eq!(attacks.count(), 13);
    }

    #[test]
    fn bishop_a1_open_board() {
        let t = Tables::get();
        // Bishop on a1 (sq 0): only NE diagonal = b2,c3,d4,e5,f6,g7,h8 = 7 squares
        let attacks = t.bishop_attacks(Square(0), Bitboard::EMPTY);
        assert_eq!(attacks.count(), 7);
        assert!(attacks.get(Square(9)));  // b2
        assert!(attacks.get(Square(63))); // h8
    }

    #[test]
    fn queen_e4_attack_count() {
        let t = Tables::get();
        // Queen on e4 (sq 28), open board: rook rays + bishop rays
        // Rook: 14, Bishop: 13 — they don't overlap, total = 27
        let attacks = t.queen_attacks(Square(28), Bitboard::EMPTY);
        assert_eq!(attacks.count(), 27);
    }
}
