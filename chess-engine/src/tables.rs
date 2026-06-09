use std::sync::OnceLock;
use crate::bitboard::Bitboard;
use crate::types::{Color, Square};

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
            bishop_magics:  stub_magic_tables(),
            rook_magics:    stub_magic_tables(),
        }
    }

    pub fn bishop_attacks(&self, sq: Square, occupancy: Bitboard) -> Bitboard {
        let entry = &self.bishop_magics[sq.0 as usize];
        if entry.attacks.is_empty() { return Bitboard::EMPTY; }
        let idx = ((occupancy & entry.mask).0.wrapping_mul(entry.magic) >> entry.shift) as usize;
        entry.attacks[idx]
    }

    pub fn rook_attacks(&self, sq: Square, occupancy: Bitboard) -> Bitboard {
        let entry = &self.rook_magics[sq.0 as usize];
        if entry.attacks.is_empty() { return Bitboard::EMPTY; }
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
}
