use crate::types::Square;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const EMPTY: Bitboard = Bitboard(0);
    pub const FULL:  Bitboard = Bitboard(u64::MAX);

    pub const RANK_1: Bitboard = Bitboard(0x00000000000000FF);
    pub const RANK_2: Bitboard = Bitboard(0x000000000000FF00);
    pub const RANK_7: Bitboard = Bitboard(0x00FF000000000000);
    pub const RANK_8: Bitboard = Bitboard(0xFF00000000000000);
    pub const FILE_A: Bitboard = Bitboard(0x0101010101010101);
    pub const FILE_H: Bitboard = Bitboard(0x8080808080808080);

    pub fn is_empty(self) -> bool { self.0 == 0 }
    pub fn count(self) -> u32 { self.0.count_ones() }

    pub fn get(self, sq: Square) -> bool { (self.0 >> sq.0) & 1 == 1 }
    pub fn set(self, sq: Square) -> Bitboard { Bitboard(self.0 | (1u64 << sq.0)) }
    pub fn clear(self, sq: Square) -> Bitboard { Bitboard(self.0 & !(1u64 << sq.0)) }
    pub fn toggle(self, sq: Square) -> Bitboard { Bitboard(self.0 ^ (1u64 << sq.0)) }

    pub fn lsb(self) -> Square { Square(self.0.trailing_zeros() as u8) }

    pub fn pop_lsb(&mut self) -> Square {
        let sq = self.lsb();
        self.0 &= self.0 - 1;
        sq
    }

    pub fn squares(self) -> BitboardIter { BitboardIter(self) }

    pub fn shift_north(self) -> Bitboard { Bitboard(self.0 << 8) }
    pub fn shift_south(self) -> Bitboard { Bitboard(self.0 >> 8) }
    pub fn shift_east(self)  -> Bitboard { Bitboard((self.0 & !Self::FILE_H.0) << 1) }
    pub fn shift_west(self)  -> Bitboard { Bitboard((self.0 & !Self::FILE_A.0) >> 1) }
    pub fn shift_ne(self)    -> Bitboard { Bitboard((self.0 & !Self::FILE_H.0) << 9) }
    pub fn shift_nw(self)    -> Bitboard { Bitboard((self.0 & !Self::FILE_A.0) << 7) }
    pub fn shift_se(self)    -> Bitboard { Bitboard((self.0 & !Self::FILE_H.0) >> 7) }
    pub fn shift_sw(self)    -> Bitboard { Bitboard((self.0 & !Self::FILE_A.0) >> 9) }
}

pub struct BitboardIter(pub Bitboard);

impl Iterator for BitboardIter {
    type Item = Square;
    fn next(&mut self) -> Option<Square> {
        if self.0.is_empty() { return None; }
        Some(self.0.pop_lsb())
    }
}

impl std::ops::BitAnd for Bitboard {
    type Output = Bitboard;
    fn bitand(self, rhs: Bitboard) -> Bitboard { Bitboard(self.0 & rhs.0) }
}
impl std::ops::BitOr for Bitboard {
    type Output = Bitboard;
    fn bitor(self, rhs: Bitboard) -> Bitboard { Bitboard(self.0 | rhs.0) }
}
impl std::ops::BitXor for Bitboard {
    type Output = Bitboard;
    fn bitxor(self, rhs: Bitboard) -> Bitboard { Bitboard(self.0 ^ rhs.0) }
}
impl std::ops::Not for Bitboard {
    type Output = Bitboard;
    fn not(self) -> Bitboard { Bitboard(!self.0) }
}
impl std::ops::BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Bitboard) { self.0 &= rhs.0; }
}
impl std::ops::BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Bitboard) { self.0 |= rhs.0; }
}
impl std::ops::BitXorAssign for Bitboard {
    fn bitxor_assign(&mut self, rhs: Bitboard) { self.0 ^= rhs.0; }
}
impl std::ops::Shl<u32> for Bitboard {
    type Output = Bitboard;
    fn shl(self, rhs: u32) -> Bitboard { Bitboard(self.0 << rhs) }
}
impl std::ops::Shr<u32> for Bitboard {
    type Output = Bitboard;
    fn shr(self, rhs: u32) -> Bitboard { Bitboard(self.0 >> rhs) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Square;

    #[test]
    fn set_and_get() {
        let bb = Bitboard::EMPTY.set(Square(4));
        assert!(bb.get(Square(4)));
        assert!(!bb.get(Square(5)));
        assert_eq!(bb.count(), 1);
    }

    #[test]
    fn pop_lsb() {
        let mut bb = Bitboard(0b1010);
        let sq = bb.pop_lsb();
        assert_eq!(sq, Square(1));
        assert_eq!(bb, Bitboard(0b1000));
    }

    #[test]
    fn iter_squares() {
        let bb = Bitboard(0b10101);
        let squares: Vec<Square> = bb.squares().collect();
        assert_eq!(squares, vec![Square(0), Square(2), Square(4)]);
    }

    #[test]
    fn bitwise_ops() {
        let a = Bitboard(0b1100);
        let b = Bitboard(0b1010);
        assert_eq!((a & b), Bitboard(0b1000));
        assert_eq!((a | b), Bitboard(0b1110));
        assert_eq!((a ^ b), Bitboard(0b0110));
        assert_eq!((!a).0 & 0xFF, 0b11110011);
    }

    #[test]
    fn shift_operations() {
        // A pawn on e2 (square 12, file 4, rank 1)
        let e2 = Bitboard::EMPTY.set(Square(12));
        // shift north = e3 (square 20)
        assert!(e2.shift_north().get(Square(20)));
        // shift east = f2 (square 13)
        assert!(e2.shift_east().get(Square(13)));
        // H-file pawn shifting east should not wrap
        let h2 = Bitboard::EMPTY.set(Square(15));
        assert!(h2.shift_east().is_empty());
    }
}
