use bevy::prelude::*;
use crate::board::{CastlingState, PlayerTurn, Taken};
use crate::pieces::{Piece, PieceColor, PieceType};
use crate::state::{AppState, GameConfig, GameMode};
use chess_engine::{
    DifficultyConfig, MoveFlag, Position, Search, SearchResult,
};

// ─── Resources ───────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct AiMovePending(pub Option<chess_engine::Move>);

#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
pub enum Difficulty {
    Principiante,
    Facil,
    #[default]
    Medio,
    Dificil,
    Pro,
}

impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Principiante => "Principiante",
            Difficulty::Facil        => "Fácil",
            Difficulty::Medio        => "Medio",
            Difficulty::Dificil      => "Difícil",
            Difficulty::Pro          => "Pro",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Difficulty::Principiante => Difficulty::Facil,
            Difficulty::Facil        => Difficulty::Medio,
            Difficulty::Medio        => Difficulty::Dificil,
            Difficulty::Dificil      => Difficulty::Pro,
            Difficulty::Pro          => Difficulty::Principiante,
        }
    }

    pub fn config(self) -> DifficultyConfig {
        match self {
            Difficulty::Principiante => DifficultyConfig { max_depth: 2, max_nodes: 50_000,    random_factor: 0.25 },
            Difficulty::Facil        => DifficultyConfig { max_depth: 3, max_nodes: 200_000,   random_factor: 0.10 },
            Difficulty::Medio        => DifficultyConfig { max_depth: 5, max_nodes: 2_000_000, random_factor: 0.00 },
            Difficulty::Dificil      => DifficultyConfig { max_depth: 7, max_nodes: 3_000_000, random_factor: 0.00 },
            Difficulty::Pro          => DifficultyConfig { max_depth: 64,max_nodes: 5_000_000, random_factor: 0.00 },
        }
    }
}

// ─── FEN builder ─────────────────────────────────────────────────────────────

pub fn build_fen(pieces: &[Piece], side_to_move: PieceColor, castling: &CastlingState) -> String {
    let mut ranks = Vec::with_capacity(8);
    // FEN starts from rank 8 (x=7) down to rank 1 (x=0)
    for rank in (0u8..8).rev() {
        let mut rank_str = String::new();
        let mut empty: u8 = 0;
        for file in 0u8..8 {
            if let Some(p) = pieces.iter().find(|p| p.x == rank && p.y == file) {
                if empty > 0 {
                    rank_str.push_str(&empty.to_string());
                    empty = 0;
                }
                let ch = match p.piece_type {
                    PieceType::King   => 'k',
                    PieceType::Queen  => 'q',
                    PieceType::Rook   => 'r',
                    PieceType::Bishop => 'b',
                    PieceType::Knight => 'n',
                    PieceType::Pawn   => 'p',
                };
                rank_str.push(if p.color == PieceColor::White { ch.to_ascii_uppercase() } else { ch });
            } else {
                empty += 1;
            }
        }
        if empty > 0 {
            rank_str.push_str(&empty.to_string());
        }
        ranks.push(rank_str);
    }
    let side = if side_to_move == PieceColor::White { "w" } else { "b" };
    format!("{} {} {} - 0 1", ranks.join("/"), side, castling.to_fen_str())
}

// ─── Systems ─────────────────────────────────────────────────────────────────

fn sync_difficulty_from_config(config: Res<GameConfig>, mut difficulty: ResMut<Difficulty>) {
    *difficulty = config.difficulty;
}

fn reset_ai_state(mut pending: ResMut<AiMovePending>) {
    *pending = AiMovePending::default();
}

fn ai_turn_trigger(
    turn: Res<PlayerTurn>,
    castling_state: Res<CastlingState>,
    difficulty: Res<Difficulty>,
    pieces_query: Query<&Piece>,
    mut pending_mut: ResMut<AiMovePending>,
    game_config: Res<GameConfig>,
) {
    if game_config.mode == GameMode::PvP { return; }
    if !turn.is_changed() { return; }
    if turn.0 != PieceColor::Black { return; }
    if pending_mut.0.is_some() { return; }

    let pieces: Vec<Piece> = pieces_query.iter().copied().collect();
    let fen = build_fen(&pieces, PieceColor::Black, &castling_state);

    let pos = match Position::from_fen(&fen) {
        Ok(p)  => p,
        Err(e) => { eprintln!("AI: invalid FEN '{}': {}", fen, e); return; }
    };

    let mut search = Search::new();
    if let SearchResult::EngineMove(mv, score) = search.best_move(&pos, &difficulty.config()) {
        eprintln!("AI: {} (score {})", mv.to_uci(), score);
        pending_mut.0 = Some(mv);
    }
}

fn ai_apply_move(
    mut commands: Commands,
    mut pending: ResMut<AiMovePending>,
    mut turn_mut: ResMut<PlayerTurn>,
    mut castling_state: ResMut<CastlingState>,
    mut pieces_query: Query<(Entity, &mut Piece)>,
) {
    // Only apply on the frame AFTER the trigger (turn is no longer "just changed")
    if turn_mut.is_changed() { return; }
    if turn_mut.0 != PieceColor::Black { return; }

    let mv = match pending.0.take() {
        Some(m) => m,
        None    => return,
    };

    let from = mv.from_sq();
    let to   = mv.to_sq();
    let flag = mv.flag();

    let from_bevy = (from.rank(), from.file());
    let to_bevy   = (to.rank(),   to.file());

    // Snapshot all pieces to avoid borrow conflicts during mutation
    let all: Vec<(Entity, Piece)> = pieces_query.iter().map(|(e, p)| (e, *p)).collect();

    // Find moving piece entity (must be Black)
    let moving_entity = match all.iter()
        .find(|(_, p)| p.x == from_bevy.0 && p.y == from_bevy.1 && p.color == PieceColor::Black)
        .map(|(e, _)| *e)
    {
        Some(e) => e,
        None => {
            eprintln!("AI: no Black piece at ({}, {})", from_bevy.0, from_bevy.1);
            return;
        }
    };

    // Handle standard capture
    let is_capture = matches!(flag,
        MoveFlag::Capture | MoveFlag::PromoKnightCapture | MoveFlag::PromoBishopCapture |
        MoveFlag::PromoRookCapture | MoveFlag::PromoQueenCapture
    );
    if is_capture {
        if let Some((captured_entity, _)) = all.iter()
            .find(|(_, p)| p.x == to_bevy.0 && p.y == to_bevy.1 && p.color == PieceColor::White)
        {
            commands.entity(*captured_entity).insert(Taken);
        }
    }

    // Handle en passant: captured pawn is one rank above destination (black captures white pawn)
    if flag == MoveFlag::EnPassant {
        let ep_rank = to_bevy.0 + 1;
        if let Some((ep_entity, _)) = all.iter()
            .find(|(_, p)| p.x == ep_rank && p.y == to_bevy.1 && p.color == PieceColor::White)
        {
            commands.entity(*ep_entity).insert(Taken);
        }
    }

    // Handle castling: also move the rook
    if flag == MoveFlag::KingSideCastle {
        // Black king-side: rook h8(7,7) → f8(7,5)
        if let Some((rook_e, _)) = all.iter()
            .find(|(_, p)| p.x == 7 && p.y == 7 && p.color == PieceColor::Black && p.piece_type == PieceType::Rook)
        {
            if let Ok((_, mut rook)) = pieces_query.get_mut(*rook_e) {
                rook.x = 7;
                rook.y = 5;
            }
        }
    }
    if flag == MoveFlag::QueenSideCastle {
        // Black queen-side: rook a8(7,0) → d8(7,3)
        if let Some((rook_e, _)) = all.iter()
            .find(|(_, p)| p.x == 7 && p.y == 0 && p.color == PieceColor::Black && p.piece_type == PieceType::Rook)
        {
            if let Ok((_, mut rook)) = pieces_query.get_mut(*rook_e) {
                rook.x = 7;
                rook.y = 3;
            }
        }
    }

    // Move the piece and handle promotion
    if let Ok((_, mut piece)) = pieces_query.get_mut(moving_entity) {
        piece.x = to_bevy.0;
        piece.y = to_bevy.1;

        // Strip castling rights for Black when king or rook moves
        match piece.piece_type {
            PieceType::King => {
                castling_state.black_kingside  = false;
                castling_state.black_queenside = false;
            }
            PieceType::Rook => {
                let origin_file = from_bevy.1;
                if origin_file == 7 { castling_state.black_kingside  = false; }
                if origin_file == 0 { castling_state.black_queenside = false; }
            }
            _ => {}
        }

        let is_promo = matches!(flag,
            MoveFlag::PromoKnight | MoveFlag::PromoBishop | MoveFlag::PromoRook | MoveFlag::PromoQueen |
            MoveFlag::PromoKnightCapture | MoveFlag::PromoBishopCapture |
            MoveFlag::PromoRookCapture | MoveFlag::PromoQueenCapture
        );
        if is_promo {
            piece.piece_type = match flag {
                MoveFlag::PromoKnight | MoveFlag::PromoKnightCapture => PieceType::Knight,
                MoveFlag::PromoBishop | MoveFlag::PromoBishopCapture => PieceType::Bishop,
                MoveFlag::PromoRook   | MoveFlag::PromoRookCapture   => PieceType::Rook,
                _                                                      => PieceType::Queen,
            };
        }
    }

    turn_mut.change();
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<AiMovePending>()
            .init_resource::<Difficulty>()
            .add_systems(OnEnter(AppState::Playing), sync_difficulty_from_config)
            .add_systems(OnEnter(AppState::Playing), reset_ai_state)
            .add_systems(Update, (ai_turn_trigger, ai_apply_move).chain()
                .run_if(in_state(AppState::Playing)));
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(color: PieceColor, piece_type: PieceType, x: u8, y: u8) -> Piece {
        Piece { color, piece_type, x, y }
    }

    #[test]
    fn starting_position_fen() {
        use PieceColor::*;
        use PieceType::*;
        let pieces = vec![
            piece(Black, Rook,   7, 0), piece(Black, Knight, 7, 1), piece(Black, Bishop, 7, 2),
            piece(Black, Queen,  7, 3), piece(Black, King,   7, 4), piece(Black, Bishop, 7, 5),
            piece(Black, Knight, 7, 6), piece(Black, Rook,   7, 7),
            piece(Black, Pawn,   6, 0), piece(Black, Pawn,   6, 1), piece(Black, Pawn,   6, 2),
            piece(Black, Pawn,   6, 3), piece(Black, Pawn,   6, 4), piece(Black, Pawn,   6, 5),
            piece(Black, Pawn,   6, 6), piece(Black, Pawn,   6, 7),
            piece(White, Pawn,   1, 0), piece(White, Pawn,   1, 1), piece(White, Pawn,   1, 2),
            piece(White, Pawn,   1, 3), piece(White, Pawn,   1, 4), piece(White, Pawn,   1, 5),
            piece(White, Pawn,   1, 6), piece(White, Pawn,   1, 7),
            piece(White, Rook,   0, 0), piece(White, Knight, 0, 1), piece(White, Bishop, 0, 2),
            piece(White, Queen,  0, 3), piece(White, King,   0, 4), piece(White, Bishop, 0, 5),
            piece(White, Knight, 0, 6), piece(White, Rook,   0, 7),
        ];
        let castling = CastlingState { white_kingside: true, white_queenside: true,
                                       black_kingside: true, black_queenside: true };
        let fen = build_fen(&pieces, PieceColor::Black, &castling);
        assert!(fen.starts_with("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq"));
    }

    #[test]
    fn empty_ranks_encode_as_numbers() {
        let pieces = vec![
            piece(PieceColor::White, PieceType::King, 0, 4),
            piece(PieceColor::Black, PieceType::King, 7, 4),
        ];
        let castling = CastlingState { white_kingside: false, white_queenside: false,
                                       black_kingside: false, black_queenside: false };
        let fen = build_fen(&pieces, PieceColor::Black, &castling);
        assert!(fen.starts_with("4k3/8/8/8/8/8/8/4K3 b -"));
    }
}
