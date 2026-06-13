use bevy::prelude::*;
use crate::board::{CastlingState, GameHistory, GameStatus, GameStatusEvent, PlayerTurn, Taken};
use crate::pieces::{Piece, PieceColor, PieceType};
use crate::adaptive_ai::AdaptiveAiProfile;
use crate::state::{AppState, GameConfig, GameMode, PvLMode};
use chess_engine::{
    DifficultyConfig, MoveFlag, Position, Search, SearchResult,
};

// ─── Resources ───────────────────────────────────────────────────────────────

/// AI execution phase. Drives the three-step pipeline:
///   Idle → WaitBeforeThink (timer) → Ready(mv) → Idle
/// The wait phase lets the player's piece animation render before the blocking search.
#[derive(Resource, Default)]
pub enum AiPhase {
    #[default]
    Idle,
    WaitBeforeThink(Timer),
    Ready(chess_engine::Move),
}

impl AiPhase {
    pub fn is_thinking(&self) -> bool {
        !matches!(self, AiPhase::Idle)
    }
}

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

    pub fn elo_estimate(self) -> i32 {
        match self {
            Difficulty::Principiante => 400,
            Difficulty::Facil        => 800,
            Difficulty::Medio        => 1200,
            Difficulty::Dificil      => 1600,
            Difficulty::Pro          => 2000,
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

    /// Minimum seconds to wait before starting the search. Lets the player's
    /// piece animation play. Easy levels use a longer delay to look "thoughtful".
    pub fn min_pre_delay_secs(self) -> f32 {
        match self {
            Difficulty::Principiante => 0.8,
            Difficulty::Facil        => 0.7,
            Difficulty::Medio        => 0.5,
            Difficulty::Dificil      => 0.3,
            Difficulty::Pro          => 0.3,
        }
    }
}

// ─── FEN builder ─────────────────────────────────────────────────────────────

pub fn build_fen_ep(
    pieces:       &[Piece],
    side_to_move: PieceColor,
    castling:     &CastlingState,
    en_passant:   Option<(u8, u8)>,
) -> String {
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
    let ep = match en_passant {
        Some((rank, file)) => format!("{}{}", (b'a' + file) as char, rank + 1),
        None => "-".to_string(),
    };
    format!("{} {} {} {} 0 1", ranks.join("/"), side, castling.to_fen_str(), ep)
}

pub fn build_fen(pieces: &[Piece], side_to_move: PieceColor, castling: &CastlingState) -> String {
    build_fen_ep(pieces, side_to_move, castling, None)
}

// ─── Systems ─────────────────────────────────────────────────────────────────

fn sync_difficulty_from_config(config: Res<GameConfig>, mut difficulty: ResMut<Difficulty>) {
    *difficulty = config.difficulty;
}

fn reset_ai_state(mut phase: ResMut<AiPhase>) {
    *phase = AiPhase::Idle;
}

/// Step 1: When the player moves and it becomes the AI's turn, start the
/// pre-think wait timer. The delay lets the player's piece animation run
/// before the blocking engine search freezes the render loop.
fn ai_schedule_think(
    turn: Res<PlayerTurn>,
    difficulty: Res<Difficulty>,
    mut phase: ResMut<AiPhase>,
    game_config: Res<GameConfig>,
) {
    if game_config.mode == GameMode::PvP { return; }
    if !matches!(*phase, AiPhase::Idle) { return; }
    if !turn.is_changed() { return; }

    let ai_color = match game_config.player_side {
        PieceColor::White => PieceColor::Black,
        PieceColor::Black => PieceColor::White,
    };
    if turn.0 != ai_color { return; }

    let delay = difficulty.min_pre_delay_secs();
    *phase = AiPhase::WaitBeforeThink(Timer::from_seconds(delay, TimerMode::Once));
}

/// Step 2: Tick the wait timer. When it expires, run the (blocking) engine
/// search and store the result. This frame will stutter on hard/pro because
/// the search blocks; that is unavoidable in single-threaded WASM.
fn ai_tick_and_compute(
    time:        Res<Time>,
    mut phase:   ResMut<AiPhase>,
    castling_state: Res<CastlingState>,
    difficulty:  Res<Difficulty>,
    pieces_query: Query<&Piece>,
    game_config: Res<GameConfig>,
    en_passant:  Res<crate::board::EnPassantTarget>,
    adaptive:    Res<AdaptiveAiProfile>,
) {
    let timer_done = match &mut *phase {
        AiPhase::WaitBeforeThink(timer) => {
            timer.tick(time.delta());
            timer.finished()
        }
        _ => return,
    };
    if !timer_done { return; }

    let ai_color = match game_config.player_side {
        PieceColor::White => PieceColor::Black,
        PieceColor::Black => PieceColor::White,
    };

    let pieces: Vec<Piece> = pieces_query.iter().copied().collect();
    let fen = build_fen_ep(&pieces, ai_color, &castling_state, en_passant.0);

    let pos = match Position::from_fen(&fen) {
        Ok(p)  => p,
        Err(e) => { eprintln!("AI: invalid FEN '{}': {}", fen, e); *phase = AiPhase::Idle; return; }
    };

    let diff_cfg = if game_config.mode == GameMode::PvL
        && game_config.pvl_mode == PvLMode::Adaptativa
        && adaptive.loaded
    {
        DifficultyConfig {
            max_depth:     adaptive.elo_to_depth(),
            max_nodes:     2_000_000,
            random_factor: 0.0,
        }
    } else {
        difficulty.config()
    };

    let mut search = if game_config.mode == GameMode::PvL
        && game_config.pvl_mode == PvLMode::Adaptativa
        && !adaptive.biases.is_empty()
    {
        Search::with_biases(&adaptive.biases)
    } else {
        Search::new()
    };

    match search.best_move(&pos, &diff_cfg) {
        SearchResult::EngineMove(mv, score) => {
            eprintln!("AI: {} (score {})", mv.to_uci(), score);
            *phase = AiPhase::Ready(mv);
        }
        _ => { *phase = AiPhase::Idle; }
    }
}

/// Step 3: Apply the computed move and advance the turn.
fn ai_apply_move(
    mut commands: Commands,
    mut phase: ResMut<AiPhase>,
    mut turn_mut: ResMut<PlayerTurn>,
    mut castling_state: ResMut<CastlingState>,
    mut pieces_query: Query<(Entity, &mut Piece)>,
    mut captured: ResMut<crate::captured::CapturedPieces>,
    mut status_ev: EventWriter<GameStatusEvent>,
    game_config: Res<GameConfig>,
    mut history: ResMut<GameHistory>,
    mut en_passant:  ResMut<crate::board::EnPassantTarget>,
    mut draw_state:  ResMut<crate::board::DrawTracking>,
) {
    let mv = match &*phase {
        AiPhase::Ready(mv) => *mv,
        _ => return,
    };

    let ai_color = match game_config.player_side {
        PieceColor::White => PieceColor::Black,
        PieceColor::Black => PieceColor::White,
    };
    let player_color = game_config.player_side;

    let from = mv.from_sq();
    let to   = mv.to_sq();
    let flag = mv.flag();

    let from_bevy = (from.rank(), from.file());
    let to_bevy   = (to.rank(),   to.file());

    let all: Vec<(Entity, Piece)> = pieces_query.iter().map(|(e, p)| (e, *p)).collect();

    let moving_entity = match all.iter()
        .find(|(_, p)| p.x == from_bevy.0 && p.y == from_bevy.1 && p.color == ai_color)
        .map(|(e, _)| *e)
    {
        Some(e) => e,
        None => {
            eprintln!("AI: no piece at ({}, {})", from_bevy.0, from_bevy.1);
            *phase = AiPhase::Idle;
            return;
        }
    };
    let moving_piece_type = all.iter()
        .find(|(e, _)| *e == moving_entity)
        .map(|(_, p)| p.piece_type);

    history.moves.push(mv);

    let mut just_captured_entity: Option<Entity> = None;

    let is_capture = matches!(flag,
        MoveFlag::Capture | MoveFlag::PromoKnightCapture | MoveFlag::PromoBishopCapture |
        MoveFlag::PromoRookCapture | MoveFlag::PromoQueenCapture
    );
    if is_capture {
        if let Some((captured_entity, captured_piece)) = all.iter()
            .find(|(_, p)| p.x == to_bevy.0 && p.y == to_bevy.1 && p.color == player_color)
        {
            captured.add(captured_piece);
            commands.entity(*captured_entity).insert(Taken);
            just_captured_entity = Some(*captured_entity);
        }
    }

    if flag == MoveFlag::EnPassant {
        let ep_rank = if ai_color == PieceColor::Black {
            to_bevy.0.saturating_add(1)
        } else {
            to_bevy.0.saturating_sub(1)
        };
        if let Some((ep_entity, ep_piece)) = all.iter()
            .find(|(_, p)| p.x == ep_rank && p.y == to_bevy.1 && p.color == player_color)
        {
            captured.add(ep_piece);
            commands.entity(*ep_entity).insert(Taken);
            just_captured_entity = just_captured_entity.or(Some(*ep_entity));
        }
    }

    // Halfmove clock: reset on pawn move or any capture (including en passant).
    let is_any_capture = is_capture || flag == MoveFlag::EnPassant;
    if moving_piece_type == Some(PieceType::Pawn) || is_any_capture {
        draw_state.halfmove_clock = 0;
    } else {
        draw_state.halfmove_clock += 1;
    }

    let castling_rank = if ai_color == PieceColor::Black { 7u8 } else { 0u8 };
    if flag == MoveFlag::KingSideCastle {
        if let Some((rook_e, _)) = all.iter()
            .find(|(_, p)| p.x == castling_rank && p.y == 7 && p.color == ai_color && p.piece_type == PieceType::Rook)
        {
            if let Ok((_, mut rook)) = pieces_query.get_mut(*rook_e) {
                rook.x = castling_rank;
                rook.y = 5;
            }
        }
    }
    if flag == MoveFlag::QueenSideCastle {
        if let Some((rook_e, _)) = all.iter()
            .find(|(_, p)| p.x == castling_rank && p.y == 0 && p.color == ai_color && p.piece_type == PieceType::Rook)
        {
            if let Ok((_, mut rook)) = pieces_query.get_mut(*rook_e) {
                rook.x = castling_rank;
                rook.y = 3;
            }
        }
    }

    if let Ok((_, mut piece)) = pieces_query.get_mut(moving_entity) {
        piece.x = to_bevy.0;
        piece.y = to_bevy.1;

        match piece.piece_type {
            PieceType::King => {
                if ai_color == PieceColor::Black {
                    castling_state.black_kingside  = false;
                    castling_state.black_queenside = false;
                } else {
                    castling_state.white_kingside  = false;
                    castling_state.white_queenside = false;
                }
            }
            PieceType::Rook => {
                let origin_file = from_bevy.1;
                if ai_color == PieceColor::Black {
                    if origin_file == 7 { castling_state.black_kingside  = false; }
                    if origin_file == 0 { castling_state.black_queenside = false; }
                } else {
                    if origin_file == 7 { castling_state.white_kingside  = false; }
                    if origin_file == 0 { castling_state.white_queenside = false; }
                }
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

    *phase = AiPhase::Idle;
    turn_mut.change();

    // Update en passant target based on the AI's move
    en_passant.0 = if mv.flag() == MoveFlag::DoublePush {
        let ep_rank = (mv.from_sq().rank() + mv.to_sq().rank()) / 2;
        Some((ep_rank, mv.to_sq().file()))
    } else {
        None
    };

    let all_pieces: Vec<Piece> = pieces_query.iter()
        .filter(|(e, _)| Some(*e) != just_captured_entity)
        .map(|(_, p)| *p)
        .collect();

    let pos_key = crate::board::position_key_pub(&all_pieces, turn_mut.0, &castling_state, en_passant.0);
    let is_draw = draw_state.record_position(pos_key);

    let pawns_valid = all_pieces.iter().all(|p| {
        p.piece_type != PieceType::Pawn || (p.x > 0 && p.x < 7)
    });
    if pawns_valid {
        let fen = build_fen_ep(&all_pieces, turn_mut.0, &castling_state, en_passant.0);
        if let Ok(pos) = Position::from_fen(&fen) {
            if pos.is_checkmate() {
                let winner = match turn_mut.0 {
                    PieceColor::White => PieceColor::Black,
                    PieceColor::Black => PieceColor::White,
                };
                status_ev.send(GameStatusEvent(GameStatus::Checkmate { winner }));
            } else if pos.is_stalemate() || is_draw {
                status_ev.send(GameStatusEvent(GameStatus::Stalemate));
            } else if pos.is_in_check() {
                status_ev.send(GameStatusEvent(GameStatus::Check));
            } else {
                status_ev.send(GameStatusEvent(GameStatus::Ok));
            }
        }
    } else {
        status_ev.send(GameStatusEvent(GameStatus::Ok));
    }
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<AiPhase>()
            .init_resource::<Difficulty>()
            .add_systems(OnEnter(AppState::Playing), sync_difficulty_from_config)
            .add_systems(OnEnter(AppState::Playing), reset_ai_state)
            .add_systems(Update,
                (ai_schedule_think, ai_tick_and_compute, ai_apply_move).chain()
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

    #[test]
    fn difficulty_min_delays_are_positive() {
        for d in [Difficulty::Principiante, Difficulty::Facil, Difficulty::Medio,
                  Difficulty::Dificil, Difficulty::Pro] {
            assert!(d.min_pre_delay_secs() > 0.0);
        }
    }

    #[test]
    fn build_fen_ep_includes_en_passant_target() {
        let pieces = vec![
            piece(PieceColor::White, PieceType::King, 0, 4),
            piece(PieceColor::Black, PieceType::King, 7, 4),
            piece(PieceColor::White, PieceType::Pawn, 3, 3),
        ];
        let castling = CastlingState {
            white_kingside: false, white_queenside: false,
            black_kingside: false, black_queenside: false,
        };
        let fen = build_fen_ep(&pieces, PieceColor::Black, &castling, Some((2, 3)));
        assert!(fen.contains("d3"), "FEN must contain 'd3', got: {}", fen);
    }
}
