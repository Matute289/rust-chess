#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Square(pub u8);

impl Square {
    pub const A1: Square = Square(0);
    pub const B1: Square = Square(1);
    pub const C1: Square = Square(2);
    pub const D1: Square = Square(3);
    pub const E1: Square = Square(4);
    pub const F1: Square = Square(5);
    pub const G1: Square = Square(6);
    pub const H1: Square = Square(7);
    pub const A8: Square = Square(56);
    pub const E8: Square = Square(60);
    pub const H8: Square = Square(63);

    pub fn from_rank_file(rank: u8, file: u8) -> Square {
        Square(rank * 8 + file)
    }

    pub fn rank(self) -> u8 { self.0 / 8 }
    pub fn file(self) -> u8 { self.0 % 8 }

    pub fn from_uci(s: &str) -> Option<Square> {
        let bytes = s.as_bytes();
        if bytes.len() < 2 { return None; }
        let file = bytes[0].checked_sub(b'a')?.min(7);
        let rank = bytes[1].checked_sub(b'1')?.min(7);
        Some(Square::from_rank_file(rank, file))
    }

    pub fn to_uci(self) -> [u8; 2] {
        [b'a' + self.file(), b'1' + self.rank()]
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let uci = self.to_uci();
        write!(f, "{}{}", uci[0] as char, uci[1] as char)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    pub fn flip(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    pub fn index(self) -> usize { self as usize }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PieceType {
    Pawn   = 0,
    Knight = 1,
    Bishop = 2,
    Rook   = 3,
    Queen  = 4,
    King   = 5,
}

impl PieceType {
    pub fn index(self) -> usize { self as usize }

    pub fn from_char(c: char) -> Option<PieceType> {
        match c.to_ascii_lowercase() {
            'p' => Some(PieceType::Pawn),
            'n' => Some(PieceType::Knight),
            'b' => Some(PieceType::Bishop),
            'r' => Some(PieceType::Rook),
            'q' => Some(PieceType::Queen),
            'k' => Some(PieceType::King),
            _ => None,
        }
    }

    pub fn to_char(self, color: Color) -> char {
        let c = match self {
            PieceType::Pawn   => 'p',
            PieceType::Knight => 'n',
            PieceType::Bishop => 'b',
            PieceType::Rook   => 'r',
            PieceType::Queen  => 'q',
            PieceType::King   => 'k',
        };
        if color == Color::White { c.to_ascii_uppercase() } else { c }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CastlingRights(pub u8);

impl CastlingRights {
    pub const NONE: CastlingRights = CastlingRights(0);
    pub const ALL:  CastlingRights = CastlingRights(0b1111);

    pub const WHITE_KINGSIDE:  u8 = 0b0001;
    pub const WHITE_QUEENSIDE: u8 = 0b0010;
    pub const BLACK_KINGSIDE:  u8 = 0b0100;
    pub const BLACK_QUEENSIDE: u8 = 0b1000;

    pub fn has(self, right: u8) -> bool { self.0 & right != 0 }
    pub fn add(self, right: u8) -> CastlingRights { CastlingRights(self.0 | right) }
    pub fn remove(self, right: u8) -> CastlingRights { CastlingRights(self.0 & !right) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_rank_file() {
        let sq = Square::from_rank_file(1, 4); // e2
        assert_eq!(sq.rank(), 1);
        assert_eq!(sq.file(), 4);
        assert_eq!(sq.0, 12);
    }

    #[test]
    fn color_flip() {
        assert_eq!(Color::White.flip(), Color::Black);
        assert_eq!(Color::Black.flip(), Color::White);
    }

    #[test]
    fn castling_rights() {
        let rights = CastlingRights::ALL;
        assert!(rights.has(CastlingRights::WHITE_KINGSIDE));
        assert!(rights.has(CastlingRights::BLACK_QUEENSIDE));
        let stripped = rights.remove(CastlingRights::WHITE_KINGSIDE);
        assert!(!stripped.has(CastlingRights::WHITE_KINGSIDE));
        assert!(stripped.has(CastlingRights::WHITE_QUEENSIDE));
    }
}
