use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use crate::ai::build_fen;
use crate::captured::{CapturedPieces, PromotionPending};
use crate::pieces::{Piece, PieceColor, PieceType};
use crate::state::{AppState, GameConfig, GameMode};
use chess_engine::{Move as EngineMove, MoveFlag, Position, Square as EngineSquare};

#[derive(Resource, Default)]
pub struct SelectedSquare {
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct SelectedPiece {
    pub entity: Option<Entity>,
}

#[derive(Component)]
pub struct Square {
    pub x: u8,
    pub y: u8,
}

impl Square {
    fn is_white(&self) -> bool {
        (self.x + self.y + 1) % 2 == 0
    }
}

#[derive(Resource, Default)]
pub struct ValidMoveSquares(pub Vec<(u8, u8)>);

#[derive(Event, Clone)]
pub struct GameStatusEvent(pub GameStatus);

#[derive(Clone, PartialEq)]
pub enum GameStatus {
    Ok,
    Check,
    Checkmate { winner: PieceColor },
    Stalemate,
}

#[derive(Component)]
pub struct BadMoveFlash(pub Timer);

#[derive(Resource, Default)]
pub struct GameHistory {
    pub initial_fen: String,
    pub moves: Vec<EngineMove>,
}

impl GameHistory {
    pub fn reset(&mut self) {
        self.initial_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string();
        self.moves.clear();
    }
}

#[derive(Resource)]
struct SquareMaterials {
    highlight_white: Handle<StandardMaterial>,
    highlight_black: Handle<StandardMaterial>,
    selected_white:  Handle<StandardMaterial>,
    selected_black:  Handle<StandardMaterial>,
    valid_white:     Handle<StandardMaterial>,
    valid_black:     Handle<StandardMaterial>,
    bad_white:       Handle<StandardMaterial>,
    bad_black:       Handle<StandardMaterial>,
    white_color:     Handle<StandardMaterial>,
    black_color:     Handle<StandardMaterial>,
}

impl FromWorld for SquareMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        SquareMaterials {
            // Hover (blue tint): white/black base with subtle blue wash
            highlight_white: materials.add(Color::rgb(0.75, 0.82, 1.00)),
            highlight_black: materials.add(Color::rgb(0.10, 0.18, 0.45)),
            // Selected (green tint)
            selected_white:  materials.add(Color::rgb(0.72, 1.00, 0.78)),
            selected_black:  materials.add(Color::rgb(0.06, 0.42, 0.16)),
            // Valid move destination (lighter green tint)
            valid_white:     materials.add(Color::rgb(0.78, 1.00, 0.78)),
            valid_black:     materials.add(Color::rgb(0.10, 0.38, 0.12)),
            // Bad move flash (red tint)
            bad_white:       materials.add(Color::rgb(1.00, 0.72, 0.72)),
            bad_black:       materials.add(Color::rgb(0.42, 0.06, 0.06)),
            // Original colors
            white_color:     materials.add(Color::rgb(1.00, 0.90, 0.90)),
            black_color:     materials.add(Color::rgb(0.00, 0.10, 0.10)),
        }
    }
}

fn create_board(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<SquareMaterials>,
) {
    let mesh = meshes.add(Mesh::from(Plane3d::default()));

    for i in 0u8..8 {
        for j in 0u8..8 {
            commands.spawn((
                PbrBundle {
                    mesh: mesh.clone(),
                    material: if (i + j + 1) % 2 == 0 {
                        materials.white_color.clone()
                    } else {
                        materials.black_color.clone()
                    },
                    transform: Transform::from_translation(Vec3::new(i as f32, 0.0, j as f32)),
                    ..default()
                },
                PickableBundle::default(),
                Square { x: i, y: j },
            ));
        }
    }
}

fn color_squares(
    selected_square: Res<SelectedSquare>,
    valid_moves:     Res<ValidMoveSquares>,
    materials:       Res<SquareMaterials>,
    time:            Res<Time>,
    mut query: Query<(Entity, &Square, &mut Handle<StandardMaterial>, Option<&PickingInteraction>, Option<&mut BadMoveFlash>)>,
    mut commands:    Commands,
) {
    for (entity, square, mut material, interaction, flash) in query.iter_mut() {
        if let Some(mut f) = flash {
            f.0.tick(time.delta());
            if !f.0.finished() {
                *material = if square.is_white() { materials.bad_white.clone() } else { materials.bad_black.clone() };
                continue;
            }
            commands.entity(entity).remove::<BadMoveFlash>();
        }

        let is_valid = valid_moves.0.contains(&(square.x, square.y));
        let is_white = square.is_white();

        *material = match interaction {
            Some(PickingInteraction::Hovered | PickingInteraction::Pressed) => {
                if is_white { materials.highlight_white.clone() } else { materials.highlight_black.clone() }
            }
            _ if Some(entity) == selected_square.entity => {
                if is_white { materials.selected_white.clone() } else { materials.selected_black.clone() }
            }
            _ if is_valid => {
                if is_white { materials.valid_white.clone() } else { materials.valid_black.clone() }
            }
            _ if is_white => materials.white_color.clone(),
            _ => materials.black_color.clone(),
        };
    }
}

fn select_square(
    mut click_events: EventReader<Pointer<Click>>,
    squares_query: Query<(Entity, &Square)>,
    pieces_query: Query<&Piece>,
    parent_query: Query<&Parent>,
    mut selected_square: ResMut<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
) {
    for event in click_events.read() {
        let target = event.target;

        // Direct square click
        if squares_query.get(target).is_ok() {
            selected_square.entity = Some(target);
            continue;
        }

        // Direct piece entity click
        let piece_opt = pieces_query.get(target).ok().or_else(|| {
            // Child mesh click — walk up one level to find the Piece parent
            parent_query
                .get(target)
                .ok()
                .and_then(|p| pieces_query.get(p.get()).ok())
        });

        if let Some(piece) = piece_opt {
            if let Some((sq_entity, _)) = squares_query
                .iter()
                .find(|(_, sq)| sq.x == piece.x && sq.y == piece.y)
            {
                selected_square.entity = Some(sq_entity);
            }
            continue;
        }

        selected_square.entity = None;
        selected_piece.entity = None;
    }
}


fn select_piece(
    selected_square:    Res<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
    mut valid_moves:    ResMut<ValidMoveSquares>,
    turn:               Res<PlayerTurn>,
    castling:           Res<CastlingState>,
    squares_query:      Query<&Square>,
    pieces_query:       Query<(Entity, &Piece)>,
    game_config:        Res<GameConfig>,
) {
    if !selected_square.is_changed() { return; }
    let square_entity = match selected_square.entity {
        Some(e) => e,
        None => { valid_moves.0.clear(); return; }
    };
    let square = match squares_query.get(square_entity) { Ok(s) => s, Err(_) => return };

    if selected_piece.entity.is_none() {
        for (piece_entity, piece) in pieces_query.iter() {
            if piece.x == square.x && piece.y == square.y && piece.color == turn.0 {
                // In PvC/PvL, only the human player's color can be selected
                let human_can_select = match game_config.mode {
                    GameMode::PvP => true,
                    GameMode::PvC | GameMode::PvL => piece.color == game_config.player_side,
                };
                if !human_can_select { break; }
                let pieces_vec: Vec<Piece> = pieces_query.iter().map(|(_, p)| *p).collect();
                selected_piece.entity = Some(piece_entity);
                valid_moves.0 = engine_valid_squares(piece, &pieces_vec, &castling, turn.0);
                break;
            }
        }
    } else {
        // Piece already selected — check if clicking a different own piece to re-select
        for (piece_entity, piece) in pieces_query.iter() {
            if piece.x == square.x && piece.y == square.y && piece.color == turn.0 {
                let human_can_select = match game_config.mode {
                    GameMode::PvP => true,
                    GameMode::PvC | GameMode::PvL => piece.color == game_config.player_side,
                };
                if !human_can_select { break; }
                if selected_piece.entity == Some(piece_entity) { break; } // same piece, no-op
                let pieces_vec: Vec<Piece> = pieces_query.iter().map(|(_, p)| *p).collect();
                selected_piece.entity = Some(piece_entity);
                valid_moves.0 = engine_valid_squares(piece, &pieces_vec, &castling, turn.0);
                break;
            }
        }
    }
}

fn move_piece(
    mut commands:       Commands,
    mut selected_square: ResMut<SelectedSquare>,
    selected_piece:     Res<SelectedPiece>,
    mut turn:           ResMut<PlayerTurn>,
    mut castling_state: ResMut<CastlingState>,
    mut captured:       ResMut<CapturedPieces>,
    mut promotion:      ResMut<PromotionPending>,
    mut valid_moves:    ResMut<ValidMoveSquares>,
    mut history:        ResMut<GameHistory>,
    squares_query:      Query<(Entity, &Square)>,
    mut pieces_query:   Query<(Entity, &mut Piece)>,
    mut reset_event:    EventWriter<ResetSelectedEvent>,
    mut status_event:   EventWriter<GameStatusEvent>,
) {
    if !selected_square.is_changed() { return; }
    if promotion.is_pending() { return; }
    if selected_piece.is_changed() { return; }  // piece just selected this frame, not a move

    let square_entity = match selected_square.entity { Some(e) => e, None => return };

    // Copy x/y values out before any mutable borrow
    let (square_x, square_y) = match squares_query.iter().find(|(e, _)| *e == square_entity) {
        Some((_, s)) => (s.x, s.y),
        None         => return,
    };

    let selected_piece_entity = match selected_piece.entity { Some(e) => e, None => return };

    let pieces_vec:        Vec<Piece>          = pieces_query.iter().map(|(_, p)| *p).collect();
    let pieces_entity_vec: Vec<(Entity, Piece)> = pieces_query.iter().map(|(e, p)| (e, *p)).collect();

    if let Ok((_, mut piece)) = pieces_query.get_mut(selected_piece_entity) {
        // Re-clicking the selected piece — do nothing
        if piece.x == square_x && piece.y == square_y {
            return;
        }
        if engine_valid_squares(&piece, &pieces_vec, &castling_state, turn.0)
            .contains(&(square_x, square_y))
        {
            // Capture
            let mut just_captured: Option<Entity> = None;
            for (other_entity, other_piece) in &pieces_entity_vec {
                if other_piece.x == square_x && other_piece.y == square_y && other_piece.color != piece.color {
                    captured.add(other_piece);
                    commands.entity(*other_entity).insert(Taken);
                    just_captured = Some(*other_entity);
                }
            }

            // Record this move in game history (before position changes)
            let mut eng_mv_flag: Option<MoveFlag> = None;
            {
                let from_eng = EngineSquare(piece.x * 8 + piece.y);
                let to_eng   = EngineSquare(square_x * 8 + square_y);
                let pre_fen  = build_fen(&pieces_vec, turn.0, &castling_state);
                if let Ok(pre_pos) = chess_engine::Position::from_fen(&pre_fen) {
                    if let Some(eng_mv) = find_engine_move(&pre_pos, from_eng, to_eng) {
                        eng_mv_flag = Some(eng_mv.flag());
                        history.moves.push(eng_mv);
                    }
                }
            }

            let origin_y = piece.y;
            let king_rank = piece.x;
            let king_color = piece.color;
            piece.x = square_x;
            piece.y = square_y;
            valid_moves.0.clear();

            // Castling: teleport the rook to its post-castling square.
            // pieces_entity_vec is a pre-move snapshot so the rook is still at its origin file.
            drop(piece);  // Release the mutable borrow on the king temporarily
            match eng_mv_flag {
                Some(MoveFlag::KingSideCastle) => {
                    if let Some((rook_e, _)) = pieces_entity_vec.iter()
                        .find(|(_, p)| p.x == king_rank && p.y == 7
                            && p.piece_type == PieceType::Rook && p.color == king_color)
                    {
                        if let Ok((_, mut rook)) = pieces_query.get_mut(*rook_e) {
                            rook.y = 5; // h-file (7) → f-file (5)
                        }
                    }
                }
                Some(MoveFlag::QueenSideCastle) => {
                    if let Some((rook_e, _)) = pieces_entity_vec.iter()
                        .find(|(_, p)| p.x == king_rank && p.y == 0
                            && p.piece_type == PieceType::Rook && p.color == king_color)
                    {
                        if let Ok((_, mut rook)) = pieces_query.get_mut(*rook_e) {
                            rook.y = 3; // a-file (0) → d-file (3)
                        }
                    }
                }
                _ => {}
            }

            if let Ok((_, mut piece)) = pieces_query.get_mut(selected_piece_entity) {
                // Update castling rights
                match (piece.color, piece.piece_type) {
                    (PieceColor::White, PieceType::King) => {
                        castling_state.white_kingside  = false;
                        castling_state.white_queenside = false;
                    }
                    (PieceColor::Black, PieceType::King) => {
                        castling_state.black_kingside  = false;
                        castling_state.black_queenside = false;
                    }
                    (PieceColor::White, PieceType::Rook) => {
                        if origin_y == 7 { castling_state.white_kingside  = false; }
                        if origin_y == 0 { castling_state.white_queenside = false; }
                    }
                    (PieceColor::Black, PieceType::Rook) => {
                        if origin_y == 7 { castling_state.black_kingside  = false; }
                        if origin_y == 0 { castling_state.black_queenside = false; }
                    }
                    _ => {}
                }

                // Pawn promotion detection
                let is_promotion = piece.piece_type == PieceType::Pawn
                    && ((piece.color == PieceColor::White && piece.x == 7)
                        || (piece.color == PieceColor::Black && piece.x == 0));

                if is_promotion {
                    let available = captured.available_for_promotion(piece.color);
                    if available.is_empty() {
                        piece.piece_type = PieceType::Queen; // auto-promote
                    } else {
                        promotion.pawn_entity = Some(selected_piece_entity);
                        promotion.color = Some(piece.color);
                        reset_event.send(ResetSelectedEvent);
                        return; // don't change turn yet — wait for promotion choice
                    }
                }

                turn.change();

                // Check / checkmate / stalemate detection via engine
                let all_pieces: Vec<Piece> = pieces_query.iter()
                    .filter(|(e, _)| Some(*e) != just_captured)
                    .map(|(_, p)| *p)
                    .collect();

                // Guard: don't call engine if any pawn is at an invalid rank (would cause panic)
                let pawns_valid = all_pieces.iter().all(|p| {
                    p.piece_type != PieceType::Pawn || (p.x > 0 && p.x < 7)
                });
                if pawns_valid {
                    let fen = build_fen(&all_pieces, turn.0, &castling_state);
                    if let Ok(pos) = Position::from_fen(&fen) {
                        if pos.is_checkmate() {
                            let winner = match turn.0 {
                                PieceColor::White => PieceColor::Black,
                                PieceColor::Black => PieceColor::White,
                            };
                            status_event.send(GameStatusEvent(GameStatus::Checkmate { winner }));
                        } else if pos.is_stalemate() {
                            status_event.send(GameStatusEvent(GameStatus::Stalemate));
                        } else if pos.is_in_check() {
                            status_event.send(GameStatusEvent(GameStatus::Check));
                        } else {
                            status_event.send(GameStatusEvent(GameStatus::Ok));
                        }
                    }
                } else {
                    status_event.send(GameStatusEvent(GameStatus::Ok));
                }
            }
        } else {
            // Bad move: flash destination square red for 0.5s
            if let Some((sq_entity, _)) = squares_query.iter().find(|(_, s)| s.x == square_x && s.y == square_y) {
                commands.entity(sq_entity).insert(BadMoveFlash(Timer::from_seconds(0.5, TimerMode::Once)));
            }
            // Restore selected_square to point back to the piece's own square so it
            // shows as selected (green) rather than the bad destination after the flash.
            if let Some((piece_sq_entity, _)) = squares_query.iter().find(|(_, s)| s.x == piece.x && s.y == piece.y) {
                selected_square.entity = Some(piece_sq_entity);
            }
            return;  // keep selection, don't reset
        }
    }
    reset_event.send(ResetSelectedEvent);
}

#[derive(Event)]
struct ResetSelectedEvent;

fn reset_selected(
    mut events: EventReader<ResetSelectedEvent>,
    mut selected_square: ResMut<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
    mut valid_moves: ResMut<ValidMoveSquares>,
) {
    for _ in events.read() {
        selected_square.entity = None;
        selected_piece.entity = None;
        valid_moves.0.clear();
    }
}

#[derive(Component)]
pub struct Taken;

fn despawn_taken_pieces(
    mut commands: Commands,
    query: Query<(Entity, &Piece), With<Taken>>,
) {
    for (entity, _piece) in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn reset_board_state(
    mut turn:            ResMut<PlayerTurn>,
    mut castling:        ResMut<CastlingState>,
    mut valid_moves:     ResMut<ValidMoveSquares>,
    mut selected_square: ResMut<SelectedSquare>,
    mut selected_piece:  ResMut<SelectedPiece>,
) {
    *turn            = PlayerTurn::default();
    *castling        = CastlingState::default();
    valid_moves.0.clear();
    selected_square.entity = None;
    selected_piece.entity  = None;
}

fn reset_game_history(mut history: ResMut<GameHistory>) {
    history.reset();
}

#[derive(Resource)]
pub struct PlayerTurn(pub PieceColor);

impl Default for PlayerTurn {
    fn default() -> Self {
        Self(PieceColor::White)
    }
}

impl PlayerTurn {
    pub fn change(&mut self) {
        self.0 = match self.0 {
            PieceColor::White => PieceColor::Black,
            PieceColor::Black => PieceColor::White,
        }
    }
}

#[derive(Resource)]
pub struct CastlingState {
    pub white_kingside:  bool,
    pub white_queenside: bool,
    pub black_kingside:  bool,
    pub black_queenside: bool,
}

impl Default for CastlingState {
    fn default() -> Self {
        Self {
            white_kingside:  true,
            white_queenside: true,
            black_kingside:  true,
            black_queenside: true,
        }
    }
}

impl CastlingState {
    pub fn to_fen_str(&self) -> String {
        let mut s = String::new();
        if self.white_kingside  { s.push('K'); }
        if self.white_queenside { s.push('Q'); }
        if self.black_kingside  { s.push('k'); }
        if self.black_queenside { s.push('q'); }
        if s.is_empty() { s.push('-'); }
        s
    }
}

fn find_engine_move(pos: &chess_engine::Position, from: EngineSquare, to: EngineSquare) -> Option<EngineMove> {
    let candidates: Vec<_> = pos.legal_moves()
        .into_iter()
        .filter(|m| m.from_sq() == from && m.to_sq() == to)
        .collect();
    candidates.iter()
        .find(|m| matches!(m.flag(), MoveFlag::PromoQueen | MoveFlag::PromoQueenCapture))
        .or_else(|| candidates.first())
        .copied()
}

fn legal_squares_for(fen: &str, rank: u8, file: u8) -> Vec<(u8, u8)> {
    let Ok(pos) = Position::from_fen(fen) else { return Vec::new() };
    let from_sq = EngineSquare(rank * 8 + file);
    let mut squares: Vec<(u8, u8)> = pos.legal_moves()
        .into_iter()
        .filter(|m| m.from_sq() == from_sq)
        .map(|m| (m.to_sq().rank(), m.to_sq().file()))
        .collect();
    squares.sort_unstable();
    squares.dedup();
    squares
}

fn engine_valid_squares(piece: &Piece, pieces_vec: &[Piece], castling: &CastlingState, turn: PieceColor) -> Vec<(u8, u8)> {
    let fen = build_fen(pieces_vec, turn, castling);
    legal_squares_for(&fen, piece.x, piece.y)
}

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<SelectedSquare>()
            .init_resource::<SelectedPiece>()
            .init_resource::<PlayerTurn>()
            .init_resource::<CastlingState>()
            .init_resource::<ValidMoveSquares>()
            .init_resource::<GameHistory>()
            .init_resource::<SquareMaterials>()
            .add_event::<ResetSelectedEvent>()
            .add_event::<GameStatusEvent>()
            .add_systems(Startup, create_board)
            .add_systems(OnEnter(AppState::Playing), (reset_board_state, reset_game_history))
            .add_systems(
                Update,
                (
                    select_square,
                    select_piece,
                    move_piece,
                    despawn_taken_pieces,
                    reset_selected,
                    color_squares,
                ).chain().run_if(in_state(AppState::Playing)),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::legal_squares_for;

    #[test]
    fn king_cannot_move_into_check() {
        // White king e1 (rank 0 file 4), Black rook e8 (rank 7 file 4)
        // King can't step to e2 because that's still on the e-file covered by Re8
        let fen = "4r3/8/8/8/8/8/8/4K3 w - - 0 1";
        let squares = legal_squares_for(fen, 0, 4);
        assert!(
            !squares.contains(&(1, 4)),
            "king must not move to e2 (covered by Re8), got {:?}", squares
        );
    }

    #[test]
    fn pinned_rook_cannot_leave_file() {
        // White: King e1, Rook e4. Black: Rook e8.
        // Re4 is absolutely pinned — it can only move along the e-file.
        let fen = "4r3/8/8/8/4R3/8/8/4K3 w - - 0 1";
        let squares = legal_squares_for(fen, 3, 4); // Re4 = rank 3, file 4
        assert!(!squares.is_empty(), "pinned rook should have moves along the pin line");
        for &(_, f) in &squares {
            assert_eq!(f, 4, "pinned rook must stay on e-file, got dest file {}", f);
        }
    }

    #[test]
    fn starting_knight_g1_has_two_moves() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let squares = legal_squares_for(fen, 0, 6); // Ng1
        assert!(squares.contains(&(2, 5)), "Ng1-f3 should be legal");   // f3 = rank 2, file 5
        assert!(squares.contains(&(2, 7)), "Ng1-h3 should be legal");   // h3 = rank 2, file 7
        assert_eq!(squares.len(), 2, "Ng1 has exactly 2 legal moves from start");
    }

    #[test]
    fn king_castling_squares_are_highlighted() {
        // Standard castling position: both sides can castle both ways
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        let squares = legal_squares_for(fen, 0, 4); // White king on e1 (rank 0, file 4)
        assert!(squares.contains(&(0, 6)), "White king must be able to castle kingside to g1");
        assert!(squares.contains(&(0, 2)), "White king must be able to castle queenside to c1");
    }
}
