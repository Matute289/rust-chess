use crate::types::{CastlingRights, Color, PieceType, Square};

// Packed u32: bits 0-5 = from, bits 6-11 = to, bits 12-15 = flags
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Move(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MoveFlag {
    Quiet              = 0,
    DoublePush         = 1,
    KingSideCastle     = 2,
    QueenSideCastle    = 3,
    Capture            = 4,   // bit 2 = capture
    EnPassant          = 5,
    // 6, 7 reserved
    PromoKnight        = 8,   // bit 3 = promotion; bits 0-1 = piece (0=N,1=B,2=R,3=Q)
    PromoBishop        = 9,
    PromoRook          = 10,
    PromoQueen         = 11,
    PromoKnightCapture = 12,  // bit 3 = promo, bit 2 = capture
    PromoBishopCapture = 13,
    PromoRookCapture   = 14,
    PromoQueenCapture  = 15,
}

impl Move {
    pub const NULL: Move = Move(0);

    pub fn new(from: Square, to: Square, flag: MoveFlag) -> Move {
        Move((from.0 as u32) | ((to.0 as u32) << 6) | ((flag as u32) << 12))
    }

    pub fn from_sq(self) -> Square { Square((self.0 & 0x3F) as u8) }
    pub fn to_sq(self)   -> Square { Square(((self.0 >> 6) & 0x3F) as u8) }

    pub fn flag(self) -> MoveFlag {
        match (self.0 >> 12) & 0xF {
            0  => MoveFlag::Quiet,
            1  => MoveFlag::DoublePush,
            2  => MoveFlag::KingSideCastle,
            3  => MoveFlag::QueenSideCastle,
            4  => MoveFlag::Capture,
            5  => MoveFlag::EnPassant,
            8  => MoveFlag::PromoKnight,
            9  => MoveFlag::PromoBishop,
            10 => MoveFlag::PromoRook,
            11 => MoveFlag::PromoQueen,
            12 => MoveFlag::PromoKnightCapture,
            13 => MoveFlag::PromoBishopCapture,
            14 => MoveFlag::PromoRookCapture,
            15 => MoveFlag::PromoQueenCapture,
            _  => MoveFlag::Quiet,
        }
    }

    // bit 2 of flags = capture
    pub fn is_capture(self) -> bool { (self.0 >> 12) & 0xF & 4 != 0 }
    // bit 3 of flags = promotion
    pub fn is_promotion(self) -> bool { (self.0 >> 12) & 0x8 != 0 }

    // bits 0-1 of flags = promotion piece (only valid when is_promotion())
    pub fn promo_piece(self) -> PieceType {
        match (self.0 >> 12) & 0x3 {
            0 => PieceType::Knight,
            1 => PieceType::Bishop,
            2 => PieceType::Rook,
            _ => PieceType::Queen,
        }
    }

    pub fn is_en_passant(self) -> bool { self.flag() == MoveFlag::EnPassant }
    pub fn is_castling(self) -> bool {
        matches!(self.flag(), MoveFlag::KingSideCastle | MoveFlag::QueenSideCastle)
    }

    pub fn to_uci(self) -> String {
        let from = self.from_sq().to_uci();
        let to   = self.to_sq().to_uci();
        let mut s = format!(
            "{}{}{}{}",
            from[0] as char, from[1] as char,
            to[0]   as char, to[1]   as char,
        );
        if self.is_promotion() {
            s.push(match self.promo_piece() {
                PieceType::Knight => 'n',
                PieceType::Bishop => 'b',
                PieceType::Rook   => 'r',
                _                 => 'q',
            });
        }
        s
    }
}

#[derive(Copy, Clone)]
pub struct SavedState {
    pub castling_rights: CastlingRights,
    pub en_passant:      Option<Square>,
    pub halfmove_clock:  u8,
    pub captured_piece:  Option<PieceType>,
    pub captured_color:  Color,
    pub hash:            u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PieceType, Square};

    #[test]
    fn move_encoding() {
        let m = Move::new(Square(12), Square(28), MoveFlag::Quiet);
        assert_eq!(m.from_sq(), Square(12));
        assert_eq!(m.to_sq(), Square(28));
        assert_eq!(m.flag(), MoveFlag::Quiet);
        assert!(!m.is_capture());
        assert!(!m.is_promotion());
    }

    #[test]
    fn capture_flag() {
        let m = Move::new(Square(0), Square(8), MoveFlag::Capture);
        assert!(m.is_capture());
        assert!(!m.is_promotion());
    }

    #[test]
    fn promotion_piece() {
        let m = Move::new(Square(48), Square(56), MoveFlag::PromoQueen);
        assert!(m.is_promotion());
        assert!(!m.is_capture());
        assert_eq!(m.promo_piece(), PieceType::Queen);
    }

    #[test]
    fn promo_capture_piece() {
        let m = Move::new(Square(48), Square(57), MoveFlag::PromoQueenCapture);
        assert!(m.is_promotion());
        assert!(m.is_capture());
        assert_eq!(m.promo_piece(), PieceType::Queen);
    }

    #[test]
    fn uci_encoding() {
        let m = Move::new(Square(12), Square(28), MoveFlag::Quiet);
        assert_eq!(m.to_uci(), "e2e4");
        let promo = Move::new(Square(48), Square(56), MoveFlag::PromoQueen);
        assert_eq!(promo.to_uci(), "a7a8q");
    }
}
