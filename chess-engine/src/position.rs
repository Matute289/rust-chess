use crate::bitboard::Bitboard;
#[allow(unused_imports)]
use crate::moves::{Move, SavedState};
use crate::types::{CastlingRights, Color, PieceType, Square};

pub struct Position {
    pub pieces:          [[Bitboard; 6]; 2],  // [Color as usize][PieceType as usize]
    pub side_to_move:    Color,
    pub castling_rights: CastlingRights,
    pub en_passant:      Option<Square>,
    pub halfmove_clock:  u8,
    pub fullmove_number: u16,
    pub hash:            u64,
}

// ── Zobrist ───────────────────────────────────────────────────────────────────

struct ZobristKeys {
    piece_sq:   [[[u64; 64]; 6]; 2],  // [color][piece][square]
    castling:   [u64; 16],             // indexed by 4-bit castling rights
    en_passant: [u64; 8],              // indexed by file
    black_move: u64,
}

fn gen_zobrist() -> ZobristKeys {
    let mut seed = 0xDEADBEEFCAFEBABEu64;
    let mut next = || -> u64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut piece_sq = [[[0u64; 64]; 6]; 2];
    for c in 0..2 { for p in 0..6 { for s in 0..64 { piece_sq[c][p][s] = next(); } } }
    let mut castling = [0u64; 16];
    for i in 0..16 { castling[i] = next(); }
    let mut en_passant = [0u64; 8];
    for f in 0..8 { en_passant[f] = next(); }
    ZobristKeys { piece_sq, castling, en_passant, black_move: next() }
}

static ZOBRIST: std::sync::OnceLock<ZobristKeys> = std::sync::OnceLock::new();
fn zobrist() -> &'static ZobristKeys { ZOBRIST.get_or_init(gen_zobrist) }

// ── Position methods ──────────────────────────────────────────────────────────

impl Position {
    pub fn occupied(&self) -> Bitboard {
        let mut bb = Bitboard::EMPTY;
        for c in 0..2 { for p in 0..6 { bb |= self.pieces[c][p]; } }
        bb
    }

    pub fn color_bb(&self, c: Color) -> Bitboard {
        let mut bb = Bitboard::EMPTY;
        for p in 0..6 { bb |= self.pieces[c as usize][p]; }
        bb
    }

    pub fn piece_at(&self, sq: Square) -> Option<(Color, PieceType)> {
        for c in [Color::White, Color::Black] {
            for p in [PieceType::Pawn, PieceType::Knight, PieceType::Bishop,
                      PieceType::Rook, PieceType::Queen, PieceType::King] {
                if self.pieces[c as usize][p as usize].get(sq) {
                    return Some((c, p));
                }
            }
        }
        None
    }

    pub fn king_sq(&self, c: Color) -> Square {
        self.pieces[c as usize][PieceType::King as usize].lsb()
    }

    pub fn piece_count(&self) -> u32 {
        self.occupied().count()
    }

    fn compute_hash(&self) -> u64 {
        let z = zobrist();
        let mut h = 0u64;
        for c in 0..2usize {
            for p in 0..6usize {
                for sq in self.pieces[c][p].squares() {
                    h ^= z.piece_sq[c][p][sq.0 as usize];
                }
            }
        }
        h ^= z.castling[self.castling_rights.0 as usize];
        if let Some(ep) = self.en_passant {
            h ^= z.en_passant[ep.file() as usize];
        }
        if self.side_to_move == Color::Black { h ^= z.black_move; }
        h
    }

    // ── FEN ──────────────────────────────────────────────────────────────────

    pub fn from_fen(fen: &str) -> Result<Position, &'static str> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 { return Err("too few FEN fields"); }

        let mut pieces = [[Bitboard::EMPTY; 6]; 2];

        let mut rank: i8 = 7;
        let mut file: i8 = 0;
        for ch in parts[0].chars() {
            match ch {
                '/' => { rank -= 1; file = 0; }
                '1'..='8' => { file += (ch as i8) - ('0' as i8); }
                _ => {
                    let color = if ch.is_uppercase() { Color::White } else { Color::Black };
                    let pt = PieceType::from_char(ch).ok_or("bad piece char")?;
                    let sq = Square::from_rank_file(rank as u8, file as u8);
                    pieces[color as usize][pt as usize] =
                        pieces[color as usize][pt as usize].set(sq);
                    file += 1;
                }
            }
        }

        let side_to_move = match parts[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _   => return Err("bad side to move"),
        };

        let mut castling_rights = CastlingRights::NONE;
        if parts[2] != "-" {
            for ch in parts[2].chars() {
                match ch {
                    'K' => castling_rights = castling_rights.add(CastlingRights::WHITE_KINGSIDE),
                    'Q' => castling_rights = castling_rights.add(CastlingRights::WHITE_QUEENSIDE),
                    'k' => castling_rights = castling_rights.add(CastlingRights::BLACK_KINGSIDE),
                    'q' => castling_rights = castling_rights.add(CastlingRights::BLACK_QUEENSIDE),
                    _   => {}
                }
            }
        }

        let en_passant = if parts[3] == "-" {
            None
        } else {
            Some(Square::from_uci(parts[3]).ok_or("bad en passant square")?)
        };

        let halfmove_clock  = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let fullmove_number = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(1);

        let mut pos = Position {
            pieces, side_to_move, castling_rights, en_passant,
            halfmove_clock, fullmove_number, hash: 0,
        };
        pos.hash = pos.compute_hash();
        Ok(pos)
    }

    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        for rank in (0..8u8).rev() {
            let mut empty = 0u8;
            for file in 0..8u8 {
                let sq = Square::from_rank_file(rank, file);
                match self.piece_at(sq) {
                    None => empty += 1,
                    Some((c, p)) => {
                        if empty > 0 { fen.push((b'0' + empty) as char); empty = 0; }
                        fen.push(p.to_char(c));
                    }
                }
            }
            if empty > 0 { fen.push((b'0' + empty) as char); }
            if rank > 0 { fen.push('/'); }
        }

        fen.push(' ');
        fen.push(if self.side_to_move == Color::White { 'w' } else { 'b' });
        fen.push(' ');

        let cr = self.castling_rights;
        if cr.0 == 0 {
            fen.push('-');
        } else {
            if cr.has(CastlingRights::WHITE_KINGSIDE)  { fen.push('K'); }
            if cr.has(CastlingRights::WHITE_QUEENSIDE) { fen.push('Q'); }
            if cr.has(CastlingRights::BLACK_KINGSIDE)  { fen.push('k'); }
            if cr.has(CastlingRights::BLACK_QUEENSIDE) { fen.push('q'); }
        }
        fen.push(' ');

        match self.en_passant {
            None     => fen.push('-'),
            Some(sq) => {
                let uci = sq.to_uci();
                fen.push(uci[0] as char);
                fen.push(uci[1] as char);
            }
        }

        fen.push(' ');
        fen.push_str(&self.halfmove_clock.to_string());
        fen.push(' ');
        fen.push_str(&self.fullmove_number.to_string());
        fen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Color, PieceType, Square};

    fn startpos() -> Position {
        Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    #[test]
    fn startpos_piece_counts() {
        let pos = startpos();
        assert_eq!(pos.pieces[Color::White as usize][PieceType::Pawn as usize].count(), 8);
        assert_eq!(pos.pieces[Color::Black as usize][PieceType::Pawn as usize].count(), 8);
        assert_eq!(pos.pieces[Color::White as usize][PieceType::King as usize].count(), 1);
        assert_eq!(pos.pieces[Color::White as usize][PieceType::Knight as usize].count(), 2);
        assert_eq!(pos.side_to_move, Color::White);
    }

    #[test]
    fn startpos_fen_roundtrip() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let pos = Position::from_fen(fen).unwrap();
        assert_eq!(pos.to_fen(), fen);
    }

    #[test]
    fn kiwipete_fen_roundtrip() {
        let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        let pos = Position::from_fen(fen).unwrap();
        assert_eq!(pos.to_fen(), fen);
    }

    #[test]
    fn en_passant_fen() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let pos = Position::from_fen(fen).unwrap();
        assert_eq!(pos.en_passant, Some(Square::from_uci("e3").unwrap()));
        assert_eq!(pos.to_fen(), fen);
    }

    #[test]
    fn no_castling_fen() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1";
        let pos = Position::from_fen(fen).unwrap();
        assert_eq!(pos.castling_rights.0, 0);
        assert_eq!(pos.to_fen(), fen);
    }

    #[test]
    fn zobrist_different_positions() {
        let pos1 = startpos();
        let fen2 = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let pos2 = Position::from_fen(fen2).unwrap();
        assert_ne!(pos1.hash, pos2.hash);
    }

    #[test]
    fn piece_at() {
        let pos = startpos();
        assert_eq!(pos.piece_at(Square::E1), Some((Color::White, PieceType::King)));
        assert_eq!(pos.piece_at(Square::A1), Some((Color::White, PieceType::Rook)));
        assert_eq!(pos.piece_at(Square::from_rank_file(4, 4)), None); // e5 is empty
    }

    #[test]
    fn king_sq() {
        let pos = startpos();
        assert_eq!(pos.king_sq(Color::White), Square::E1);
        assert_eq!(pos.king_sq(Color::Black), Square::E8);
    }
}
