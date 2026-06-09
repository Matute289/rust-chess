use crate::bitboard::Bitboard;
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

    pub fn make_move_mut(&mut self, m: Move) -> SavedState {
        let z = zobrist();
        let us   = self.side_to_move;
        let them = us.flip();
        let from = m.from_sq();
        let to   = m.to_sq();

        // Snapshot state before changes (for unmake)
        let saved_castling = self.castling_rights;
        let saved_ep       = self.en_passant;
        let saved_clock    = self.halfmove_clock;
        let saved_hash     = self.hash;

        // Find moving piece
        let mut moving_piece = PieceType::Pawn;
        for pt in [PieceType::Pawn, PieceType::Knight, PieceType::Bishop,
                   PieceType::Rook, PieceType::Queen, PieceType::King] {
            if self.pieces[us as usize][pt as usize].get(from) {
                moving_piece = pt;
                break;
            }
        }

        // Remove piece from source
        self.pieces[us as usize][moving_piece as usize] =
            self.pieces[us as usize][moving_piece as usize].clear(from);
        self.hash ^= z.piece_sq[us as usize][moving_piece as usize][from.0 as usize];

        // Clear old en-passant from hash
        if let Some(ep) = self.en_passant {
            self.hash ^= z.en_passant[ep.file() as usize];
        }
        self.en_passant = None;

        // Remove captured piece (ordinary capture, not ep)
        let mut captured = None;
        let flag = m.flag();
        if m.is_capture() && flag != crate::moves::MoveFlag::EnPassant {
            for pt in [PieceType::Pawn, PieceType::Knight, PieceType::Bishop,
                       PieceType::Rook, PieceType::Queen, PieceType::King] {
                if self.pieces[them as usize][pt as usize].get(to) {
                    self.pieces[them as usize][pt as usize] =
                        self.pieces[them as usize][pt as usize].clear(to);
                    self.hash ^= z.piece_sq[them as usize][pt as usize][to.0 as usize];
                    captured = Some(pt);
                    break;
                }
            }
        }

        // En passant capture: remove the captured pawn (it's behind the to-square)
        if flag == crate::moves::MoveFlag::EnPassant {
            // Captured pawn is on same rank as from, same file as to
            let ep_pawn_sq = Square::from_rank_file(from.rank(), to.file());
            self.pieces[them as usize][PieceType::Pawn as usize] =
                self.pieces[them as usize][PieceType::Pawn as usize].clear(ep_pawn_sq);
            self.hash ^= z.piece_sq[them as usize][PieceType::Pawn as usize][ep_pawn_sq.0 as usize];
            captured = Some(PieceType::Pawn);
        }

        // Place piece at destination (promotion changes the piece type)
        let placed_piece = if m.is_promotion() { m.promo_piece() } else { moving_piece };
        self.pieces[us as usize][placed_piece as usize] =
            self.pieces[us as usize][placed_piece as usize].set(to);
        self.hash ^= z.piece_sq[us as usize][placed_piece as usize][to.0 as usize];

        // Castling: also move the rook
        match flag {
            crate::moves::MoveFlag::KingSideCastle => {
                let (rf, rt) = if us == Color::White { (Square(7), Square(5)) } else { (Square(63), Square(61)) };
                self.pieces[us as usize][PieceType::Rook as usize] =
                    self.pieces[us as usize][PieceType::Rook as usize].clear(rf).set(rt);
                self.hash ^= z.piece_sq[us as usize][PieceType::Rook as usize][rf.0 as usize];
                self.hash ^= z.piece_sq[us as usize][PieceType::Rook as usize][rt.0 as usize];
            }
            crate::moves::MoveFlag::QueenSideCastle => {
                let (rf, rt) = if us == Color::White { (Square(0), Square(3)) } else { (Square(56), Square(59)) };
                self.pieces[us as usize][PieceType::Rook as usize] =
                    self.pieces[us as usize][PieceType::Rook as usize].clear(rf).set(rt);
                self.hash ^= z.piece_sq[us as usize][PieceType::Rook as usize][rf.0 as usize];
                self.hash ^= z.piece_sq[us as usize][PieceType::Rook as usize][rt.0 as usize];
            }
            _ => {}
        }

        // Double push: set new en-passant square
        if flag == crate::moves::MoveFlag::DoublePush {
            let ep_sq = Square::from_rank_file((from.rank() + to.rank()) / 2, from.file());
            self.en_passant = Some(ep_sq);
            self.hash ^= z.en_passant[ep_sq.file() as usize];
        }

        // Update castling rights (any move from/to rook or king squares clears rights)
        self.hash ^= z.castling[self.castling_rights.0 as usize];
        const CR_MASK: [u8; 64] = {
            let mut m = [0xFFu8; 64];
            m[0]  &= !CastlingRights::WHITE_QUEENSIDE;
            m[4]  &= !(CastlingRights::WHITE_KINGSIDE | CastlingRights::WHITE_QUEENSIDE);
            m[7]  &= !CastlingRights::WHITE_KINGSIDE;
            m[56] &= !CastlingRights::BLACK_QUEENSIDE;
            m[60] &= !(CastlingRights::BLACK_KINGSIDE | CastlingRights::BLACK_QUEENSIDE);
            m[63] &= !CastlingRights::BLACK_KINGSIDE;
            m
        };
        self.castling_rights = CastlingRights(
            self.castling_rights.0 & CR_MASK[from.0 as usize] & CR_MASK[to.0 as usize]
        );
        self.hash ^= z.castling[self.castling_rights.0 as usize];

        // Halfmove clock
        self.halfmove_clock = if m.is_capture() || moving_piece == PieceType::Pawn {
            0
        } else {
            self.halfmove_clock.saturating_add(1)
        };

        // Flip side to move
        if us == Color::Black { self.fullmove_number += 1; }
        self.side_to_move = them;
        self.hash ^= z.black_move;

        SavedState {
            castling_rights: saved_castling,
            en_passant: saved_ep,
            halfmove_clock: saved_clock,
            captured_piece: captured,
            captured_color: them,
            hash: saved_hash,
        }
    }

    pub fn unmake_move_mut(&mut self, m: Move, saved: SavedState) {
        // Flip back
        self.side_to_move = self.side_to_move.flip();
        if self.side_to_move == Color::Black { self.fullmove_number -= 1; }

        let us   = self.side_to_move;
        let them = us.flip();
        let from = m.from_sq();
        let to   = m.to_sq();

        // Restore saved state (hash restored entirely from saved.hash)
        self.castling_rights = saved.castling_rights;
        self.en_passant      = saved.en_passant;
        self.halfmove_clock  = saved.halfmove_clock;
        self.hash            = saved.hash;

        // Find what is on the 'to' square now (the placed piece after make)
        let placed_piece = if m.is_promotion() {
            m.promo_piece()
        } else {
            let mut pt = PieceType::Pawn;
            for p in [PieceType::Pawn, PieceType::Knight, PieceType::Bishop,
                      PieceType::Rook, PieceType::Queen, PieceType::King] {
                if self.pieces[us as usize][p as usize].get(to) { pt = p; break; }
            }
            pt
        };
        let moving_piece = if m.is_promotion() { PieceType::Pawn } else { placed_piece };

        // Move piece back to origin
        self.pieces[us as usize][placed_piece as usize] =
            self.pieces[us as usize][placed_piece as usize].clear(to);
        self.pieces[us as usize][moving_piece as usize] =
            self.pieces[us as usize][moving_piece as usize].set(from);

        // Restore captured piece
        if let Some(cap_pt) = saved.captured_piece {
            let cap_sq = if m.is_en_passant() {
                Square::from_rank_file(from.rank(), to.file())
            } else {
                to
            };
            self.pieces[them as usize][cap_pt as usize] =
                self.pieces[them as usize][cap_pt as usize].set(cap_sq);
        }

        // Undo castling rook move
        let flag = m.flag();
        match flag {
            crate::moves::MoveFlag::KingSideCastle => {
                let (rf, rt) = if us == Color::White { (Square(7), Square(5)) } else { (Square(63), Square(61)) };
                self.pieces[us as usize][PieceType::Rook as usize] =
                    self.pieces[us as usize][PieceType::Rook as usize].clear(rt).set(rf);
            }
            crate::moves::MoveFlag::QueenSideCastle => {
                let (rf, rt) = if us == Color::White { (Square(0), Square(3)) } else { (Square(56), Square(59)) };
                self.pieces[us as usize][PieceType::Rook as usize] =
                    self.pieces[us as usize][PieceType::Rook as usize].clear(rt).set(rf);
            }
            _ => {}
        }
    }

    pub fn make_move(&self, m: Move) -> Position {
        let mut pos = Position {
            pieces:          self.pieces,
            side_to_move:    self.side_to_move,
            castling_rights: self.castling_rights,
            en_passant:      self.en_passant,
            halfmove_clock:  self.halfmove_clock,
            fullmove_number: self.fullmove_number,
            hash:            self.hash,
        };
        pos.make_move_mut(m);
        pos
    }

    pub fn legal_moves(&self) -> Vec<crate::moves::Move> {
        crate::movegen::MoveGen::legal(self)
    }

    pub fn perft(&self, depth: u8) -> u64 {
        if depth == 0 { return 1; }
        let moves = self.legal_moves();
        if depth == 1 { return moves.len() as u64; }
        let mut nodes = 0u64;
        for m in moves {
            nodes += self.make_move(m).perft(depth - 1);
        }
        nodes
    }

    pub fn perft_divide(&self, depth: u8) -> Vec<(String, u64)> {
        self.legal_moves()
            .into_iter()
            .map(|m| (m.to_uci(), self.make_move(m).perft(depth - 1)))
            .collect()
    }

    /// Returns a copy of this position with the turn passed to the opponent (no piece moved).
    /// Used for null-move pruning in search. Clears en-passant square.
    pub fn null_move(&self) -> Position {
        let z = zobrist();
        let mut pos = Position {
            pieces:          self.pieces,
            side_to_move:    self.side_to_move.flip(),
            castling_rights: self.castling_rights,
            en_passant:      None,
            halfmove_clock:  self.halfmove_clock,
            fullmove_number: self.fullmove_number,
            hash:            self.hash,
        };
        // Update hash: flip side-to-move
        pos.hash ^= z.black_move;
        // Remove old en-passant from hash if it existed
        if let Some(ep) = self.en_passant {
            pos.hash ^= z.en_passant[ep.file() as usize];
        }
        pos
    }

    pub fn is_in_check(&self) -> bool {
        crate::movegen::MoveGen::is_attacked(self, self.king_sq(self.side_to_move), self.side_to_move.flip())
    }

    pub fn is_checkmate(&self) -> bool {
        self.is_in_check() && self.legal_moves().is_empty()
    }

    pub fn is_stalemate(&self) -> bool {
        !self.is_in_check() && self.legal_moves().is_empty()
    }

    pub fn is_fifty_move_draw(&self) -> bool {
        self.halfmove_clock >= 100
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

    #[test]
    fn make_e2e4() {
        let mut pos = startpos();
        let orig_hash = pos.hash;
        // e2=12, e4=28
        let m = crate::moves::Move::new(Square(12), Square(28), crate::moves::MoveFlag::DoublePush);
        let saved = pos.make_move_mut(m);
        assert_eq!(pos.side_to_move, Color::Black);
        assert!(pos.pieces[Color::White as usize][PieceType::Pawn as usize].get(Square(28)));
        assert!(!pos.pieces[Color::White as usize][PieceType::Pawn as usize].get(Square(12)));
        assert_eq!(pos.en_passant, Some(Square::from_uci("e3").unwrap())); // e3=20
        assert_ne!(pos.hash, orig_hash);
        pos.unmake_move_mut(m, saved);
        assert_eq!(pos.side_to_move, Color::White);
        assert!(pos.pieces[Color::White as usize][PieceType::Pawn as usize].get(Square(12)));
        assert!(!pos.pieces[Color::White as usize][PieceType::Pawn as usize].get(Square(28)));
        assert_eq!(pos.en_passant, None);
        assert_eq!(pos.hash, orig_hash);
    }

    #[test]
    fn make_capture() {
        // After 1.e4 e5, white pawn on e4 captures black pawn on e5
        let mut pos = startpos();
        let m1 = crate::moves::Move::new(Square(12), Square(28), crate::moves::MoveFlag::DoublePush);
        pos.make_move_mut(m1);
        let m2 = crate::moves::Move::new(Square(52), Square(36), crate::moves::MoveFlag::DoublePush);
        pos.make_move_mut(m2);
        let m3 = crate::moves::Move::new(Square(28), Square(36), crate::moves::MoveFlag::Capture);
        pos.make_move_mut(m3);
        // Black pawn gone from e5 (36), white pawn on e5 (36)
        assert!(!pos.pieces[Color::Black as usize][PieceType::Pawn as usize].get(Square(36)));
        assert!(pos.pieces[Color::White as usize][PieceType::Pawn as usize].get(Square(36)));
    }

    #[test]
    fn make_unmake_hash_roundtrip() {
        let mut pos = Position::from_fen(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"
        ).unwrap();
        let orig_hash = pos.hash;
        let m = crate::moves::Move::new(Square(4), Square(6), crate::moves::MoveFlag::KingSideCastle); // e1g1
        let saved = pos.make_move_mut(m);
        pos.unmake_move_mut(m, saved);
        assert_eq!(pos.hash, orig_hash);
    }

    #[test]
    fn perft_startpos_d1() {
        assert_eq!(startpos().perft(1), 20);
    }

    #[test]
    fn perft_startpos_d2() {
        assert_eq!(startpos().perft(2), 400);
    }

    #[test]
    fn perft_startpos_d3() {
        assert_eq!(startpos().perft(3), 8_902);
    }

    #[test]
    fn make_move_immutable() {
        let pos = startpos();
        let m = crate::moves::Move::new(Square(12), Square(28), crate::moves::MoveFlag::DoublePush);
        let pos2 = pos.make_move(m);
        // Original unchanged
        assert_eq!(pos.side_to_move, Color::White);
        assert!(pos.pieces[Color::White as usize][PieceType::Pawn as usize].get(Square(12)));
        // New position updated
        assert_eq!(pos2.side_to_move, Color::Black);
        assert!(pos2.pieces[Color::White as usize][PieceType::Pawn as usize].get(Square(28)));
    }
}
