use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use std::collections::HashMap;
use crate::ai::{build_fen, build_fen_ep};
use crate::captured::{CapturedPieces, PromotionPending};
use crate::pieces::{Piece, PieceColor, PieceType};
use crate::state::{AppState, GameConfig, GameMode, LessonSetup, Suggestion};
use chess_engine::{Move as EngineMove, MoveFlag, Position, Square as EngineSquare};

/// Bundled SystemParam for lesson-mode logic in `move_piece`.
/// Groups 3 resources into 1 param to stay within Bevy's 16-param system limit.
#[derive(SystemParam)]
struct LessonParams<'w> {
    game_config:  Res<'w, GameConfig>,
    lesson_setup: Res<'w, LessonSetup>,
    history:      ResMut<'w, GameHistory>,
}

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

#[derive(Clone, Copy, PartialEq)]
pub enum CastleSide { Kingside, Queenside }

#[derive(Resource, Default)]
pub struct CastlingPending {
    pub king_entity: Option<Entity>,
    pub side:        Option<CastleSide>,
}

impl CastlingPending {
    pub fn is_pending(&self) -> bool { self.king_entity.is_some() }
}

#[derive(Component)] pub struct CastleConfirmRoot;
#[derive(Component)] pub struct BtnCastle;

#[derive(Resource, Default)]
pub struct EnPassantTarget(pub Option<(u8, u8)>);

#[derive(Resource, Default)]
pub struct DrawTracking {
    pub halfmove_clock:   u32,
    pub position_history: HashMap<String, u8>,
}

impl DrawTracking {
    pub fn reset(&mut self) {
        self.halfmove_clock = 0;
        self.position_history.clear();
    }

    pub fn record_position(&mut self, key: String) -> bool {
        let count = self.position_history.entry(key).or_insert(0);
        *count += 1;
        self.halfmove_clock >= 100 || *count >= 3
    }
}

fn position_key(pieces: &[Piece], side: PieceColor, castling: &CastlingState, ep: Option<(u8, u8)>) -> String {
    let fen = build_fen_ep(pieces, side, castling, ep);
    fen.split(' ').take(4).collect::<Vec<_>>().join(" ")
}

pub fn position_key_pub(pieces: &[Piece], side: PieceColor, castling: &CastlingState, ep: Option<(u8, u8)>) -> String {
    position_key(pieces, side, castling, ep)
}

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
    highlight_white:   Handle<StandardMaterial>,
    highlight_black:   Handle<StandardMaterial>,
    selected_white:    Handle<StandardMaterial>,
    selected_black:    Handle<StandardMaterial>,
    valid_white:       Handle<StandardMaterial>,
    valid_black:       Handle<StandardMaterial>,
    bad_white:         Handle<StandardMaterial>,
    bad_black:         Handle<StandardMaterial>,
    suggest_from_white: Handle<StandardMaterial>,
    suggest_from_black: Handle<StandardMaterial>,
    suggest_to_white:   Handle<StandardMaterial>,
    suggest_to_black:   Handle<StandardMaterial>,
    white_color:       Handle<StandardMaterial>,
    black_color:       Handle<StandardMaterial>,
}

impl FromWorld for SquareMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        SquareMaterials {
            // Hover (blue tint)
            highlight_white: materials.add(Color::rgb(0.75, 0.82, 1.00)),
            highlight_black: materials.add(Color::rgb(0.10, 0.18, 0.45)),
            // Selected (green tint)
            selected_white:  materials.add(Color::rgb(0.72, 1.00, 0.78)),
            selected_black:  materials.add(Color::rgb(0.06, 0.42, 0.16)),
            // Valid move destination
            valid_white:     materials.add(Color::rgb(0.78, 1.00, 0.78)),
            valid_black:     materials.add(Color::rgb(0.10, 0.38, 0.12)),
            // Bad move flash (red)
            bad_white:       materials.add(Color::rgb(1.00, 0.72, 0.72)),
            bad_black:       materials.add(Color::rgb(0.42, 0.06, 0.06)),
            // Suggestion: from-square (bright blue-purple)
            suggest_from_white: materials.add(Color::rgb(0.45, 0.55, 1.00)),
            suggest_from_black: materials.add(Color::rgb(0.18, 0.28, 0.80)),
            // Suggestion: to-square (light cyan)
            suggest_to_white:   materials.add(Color::rgb(0.60, 0.88, 1.00)),
            suggest_to_black:   materials.add(Color::rgb(0.05, 0.38, 0.58)),
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
    suggestion:      Res<Suggestion>,
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

        let is_valid        = valid_moves.0.contains(&(square.x, square.y));
        let is_white        = square.is_white();
        let is_suggest_from = suggestion.from_sq.map_or(false, |(f, r)| f == square.x && r == square.y);
        let is_suggest_to   = suggestion.to_sq.map_or(false,   |(f, r)| f == square.x && r == square.y);

        *material = match interaction {
            Some(PickingInteraction::Hovered | PickingInteraction::Pressed) => {
                if is_white { materials.highlight_white.clone() } else { materials.highlight_black.clone() }
            }
            _ if Some(entity) == selected_square.entity => {
                if is_white { materials.selected_white.clone() } else { materials.selected_black.clone() }
            }
            _ if is_suggest_from => {
                if is_white { materials.suggest_from_white.clone() } else { materials.suggest_from_black.clone() }
            }
            _ if is_suggest_to => {
                if is_white { materials.suggest_to_white.clone() } else { materials.suggest_to_black.clone() }
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
    en_passant:         Res<EnPassantTarget>,
    mut pending_castle: ResMut<CastlingPending>,
    mut reset_event:    EventWriter<ResetSelectedEvent>,
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
                    GameMode::PvC | GameMode::PvL | GameMode::Lesson => piece.color == game_config.player_side,
                };
                if !human_can_select { break; }
                let pieces_vec: Vec<Piece> = pieces_query.iter().map(|(_, p)| *p).collect();
                *pending_castle = CastlingPending::default();
                selected_piece.entity = Some(piece_entity);
                valid_moves.0 = engine_valid_squares(piece, &pieces_vec, &castling, turn.0, en_passant.0);
                break;
            }
        }
    } else {
        // Castle gesture: king↔rook tapped in any order sets CastlingPending
        if let Some(sel_ent) = selected_piece.entity {
            let human_can_castle = match game_config.mode {
                GameMode::PvP => true,
                GameMode::PvC | GameMode::PvL | GameMode::Lesson => turn.0 == game_config.player_side,
            };
            if human_can_castle {
                if let Some((_, sel_p)) = pieces_query.iter().find(|(e, _)| *e == sel_ent) {
                    if let Some((clicked_ent, clicked_p)) = pieces_query.iter()
                        .find(|(_, p)| p.x == square.x && p.y == square.y && p.color == sel_p.color)
                    {
                        let pair = match (sel_p.piece_type, clicked_p.piece_type) {
                            (PieceType::King, PieceType::Rook) => Some((sel_ent,    *sel_p,    *clicked_p)),
                            (PieceType::Rook, PieceType::King) => Some((clicked_ent, *clicked_p, *sel_p)),
                            _ => None,
                        };
                        if let Some((king_ent, king, rook)) = pair {
                            if king.x == rook.x {
                                let right_ok = match (rook.color, rook.y) {
                                    (PieceColor::White, 7) => castling.white_kingside,
                                    (PieceColor::White, 0) => castling.white_queenside,
                                    (PieceColor::Black, 7) => castling.black_kingside,
                                    (PieceColor::Black, 0) => castling.black_queenside,
                                    _ => false,
                                };
                                if right_ok {
                                    let dest_file = if rook.y == 7 { 6u8 } else { 2u8 };
                                    let pieces_vec: Vec<Piece> = pieces_query.iter().map(|(_, p)| *p).collect();
                                    if engine_valid_squares(&king, &pieces_vec, &castling, turn.0, en_passant.0)
                                        .contains(&(king.x, dest_file))
                                    {
                                        let side = if rook.y == 7 { CastleSide::Kingside } else { CastleSide::Queenside };
                                        pending_castle.king_entity = Some(king_ent);
                                        pending_castle.side = Some(side);
                                        reset_event.send(ResetSelectedEvent);
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Piece already selected — check if clicking a different own piece to re-select
        for (piece_entity, piece) in pieces_query.iter() {
            if piece.x == square.x && piece.y == square.y && piece.color == turn.0 {
                let human_can_select = match game_config.mode {
                    GameMode::PvP => true,
                    GameMode::PvC | GameMode::PvL | GameMode::Lesson => piece.color == game_config.player_side,
                };
                if !human_can_select { break; }
                if selected_piece.entity == Some(piece_entity) { break; } // same piece, no-op
                let pieces_vec: Vec<Piece> = pieces_query.iter().map(|(_, p)| *p).collect();
                selected_piece.entity = Some(piece_entity);
                valid_moves.0 = engine_valid_squares(piece, &pieces_vec, &castling, turn.0, en_passant.0);
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
    mut pending_castle: ResMut<CastlingPending>,
    mut en_passant:     ResMut<EnPassantTarget>,
    mut valid_moves:    ResMut<ValidMoveSquares>,
    mut draw_state:     ResMut<DrawTracking>,
    mut lesson:         LessonParams,
    squares_query:      Query<(Entity, &Square)>,
    mut pieces_query:   Query<(Entity, &mut Piece)>,
    mut reset_event:    EventWriter<ResetSelectedEvent>,
    mut status_event:   EventWriter<GameStatusEvent>,
) {
    if !selected_square.is_changed() { return; }
    if promotion.is_pending() { return; }
    if pending_castle.is_pending() { return; }
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
        if engine_valid_squares(&piece, &pieces_vec, &castling_state, turn.0, en_passant.0)
            .contains(&(square_x, square_y))
        {
            // In lesson mode, only the exact answer move is allowed.
            if lesson.game_config.mode == GameMode::Lesson
                && lesson.lesson_setup.answer_uci.len() >= 4
            {
                let b = lesson.lesson_setup.answer_uci.as_bytes();
                let is_answer = piece.x == b[1] - b'1'
                    && piece.y == b[0] - b'a'
                    && square_x == b[3] - b'1'
                    && square_y == b[2] - b'a';
                if !is_answer {
                    if let Some((sq_e, _)) = squares_query.iter().find(|(_, s)| s.x == square_x && s.y == square_y) {
                        commands.entity(sq_e).insert(BadMoveFlash(Timer::from_seconds(0.5, TimerMode::Once)));
                    }
                    if let Some((p_sq_e, _)) = squares_query.iter().find(|(_, s)| s.x == piece.x && s.y == piece.y) {
                        selected_square.entity = Some(p_sq_e);
                    }
                    return;
                }
            }

            // Castling moves must go through the confirmation button — intercept here
            if piece.piece_type == PieceType::King {
                let from_eng = EngineSquare(piece.x * 8 + piece.y);
                let to_eng   = EngineSquare(square_x * 8 + square_y);
                let pre_fen  = build_fen_ep(&pieces_vec, turn.0, &castling_state, en_passant.0);
                if let Ok(pre_pos) = chess_engine::Position::from_fen(&pre_fen) {
                    if let Some(eng_mv) = find_engine_move(&pre_pos, from_eng, to_eng) {
                        let castle_side = match eng_mv.flag() {
                            MoveFlag::KingSideCastle  => Some(CastleSide::Kingside),
                            MoveFlag::QueenSideCastle => Some(CastleSide::Queenside),
                            _ => None,
                        };
                        if let Some(side) = castle_side {
                            pending_castle.king_entity = selected_piece.entity;
                            pending_castle.side = Some(side);
                            reset_event.send(ResetSelectedEvent);
                            return;
                        }
                    }
                }
            }

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
                let pre_fen  = build_fen_ep(&pieces_vec, turn.0, &castling_state, en_passant.0);
                if let Ok(pre_pos) = chess_engine::Position::from_fen(&pre_fen) {
                    if let Some(eng_mv) = find_engine_move(&pre_pos, from_eng, to_eng) {
                        eng_mv_flag = Some(eng_mv.flag());
                        lesson.history.moves.push(eng_mv);
                    }
                }
            }

            let origin_y  = piece.y;
            let king_rank  = piece.x;
            let king_color = piece.color;
            let piece_type = piece.piece_type;
            piece.x = square_x;
            piece.y = square_y;
            valid_moves.0.clear();

            // En passant capture: the captured pawn is NOT at the destination but at the
            // capturing pawn's origin rank, same file as destination. Remove it here.
            if matches!(eng_mv_flag, Some(MoveFlag::EnPassant)) {
                if let Some((ep_ent, ep_piece)) = pieces_entity_vec.iter()
                    .find(|(_, p)| p.x == king_rank && p.y == square_y && p.color != king_color)
                {
                    captured.add(ep_piece);
                    commands.entity(*ep_ent).insert(Taken);
                    just_captured = Some(*ep_ent);
                }
            }

            // Set/clear en passant target for opponent's next turn.
            en_passant.0 = if piece_type == PieceType::Pawn
                && (square_x as i8 - king_rank as i8).abs() == 2
            {
                Some(((king_rank + square_x) / 2, square_y))
            } else {
                None
            };

            // Halfmove clock: reset on pawn move or capture, increment otherwise.
            if piece_type == PieceType::Pawn || just_captured.is_some() {
                draw_state.halfmove_clock = 0;
            } else {
                draw_state.halfmove_clock += 1;
            }

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

                // Record position for threefold-repetition and 50-move tracking.
                let pos_key = position_key(&all_pieces, turn.0, &castling_state, en_passant.0);
                let is_draw = draw_state.record_position(pos_key);

                // Guard: don't call engine if any pawn is at an invalid rank (would cause panic)
                let pawns_valid = all_pieces.iter().all(|p| {
                    p.piece_type != PieceType::Pawn || (p.x > 0 && p.x < 7)
                });
                if pawns_valid {
                    let fen = build_fen_ep(&all_pieces, turn.0, &castling_state, en_passant.0);
                    if let Ok(pos) = Position::from_fen(&fen) {
                        if pos.is_checkmate() {
                            let winner = match turn.0 {
                                PieceColor::White => PieceColor::Black,
                                PieceColor::Black => PieceColor::White,
                            };
                            status_event.send(GameStatusEvent(GameStatus::Checkmate { winner }));
                        } else if pos.is_stalemate() || is_draw {
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
    mut turn:             ResMut<PlayerTurn>,
    mut castling:         ResMut<CastlingState>,
    mut valid_moves:      ResMut<ValidMoveSquares>,
    mut selected_square:  ResMut<SelectedSquare>,
    mut selected_piece:   ResMut<SelectedPiece>,
    mut pending_castle:   ResMut<CastlingPending>,
    mut en_passant:       ResMut<EnPassantTarget>,
    mut draw_state:       ResMut<DrawTracking>,
) {
    *turn            = PlayerTurn::default();
    *castling        = CastlingState::default();
    valid_moves.0.clear();
    selected_square.entity = None;
    selected_piece.entity  = None;
    *pending_castle  = CastlingPending::default();
    en_passant.0     = None;
    draw_state.reset();
}

fn reset_game_history(
    mut history:  ResMut<GameHistory>,
    game_config:  Res<GameConfig>,
    lesson_setup: Res<LessonSetup>,
) {
    history.reset();
    if game_config.mode == GameMode::Lesson && !lesson_setup.fen.is_empty() {
        history.initial_fen = lesson_setup.fen.clone();
    }
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

fn engine_valid_squares(
    piece:      &Piece,
    pieces_vec: &[Piece],
    castling:   &CastlingState,
    turn:       PieceColor,
    en_passant: Option<(u8, u8)>,
) -> Vec<(u8, u8)> {
    let fen = build_fen_ep(pieces_vec, turn, castling, en_passant);
    legal_squares_for(&fen, piece.x, piece.y)
}

fn show_castling_button(
    mut commands: Commands,
    pending:      Res<CastlingPending>,
    root_q:       Query<Entity, With<CastleConfirmRoot>>,
    asset_server: Res<AssetServer>,
) {
    if !pending.is_changed() { return; }
    for e in root_q.iter() { commands.entity(e).despawn_recursive(); }
    if !pending.is_pending() { return; }

    let font: Handle<Font> = asset_server.load("fonts/DejaVuSans-Bold.ttf");
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type:   PositionType::Absolute,
                width:           Val::Percent(100.0),
                bottom:          Val::Px(120.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        },
        CastleConfirmRoot,
    ))
    .with_children(|p| {
        p.spawn((
            ButtonBundle {
                style: Style {
                    width:           Val::Px(220.0),
                    height:          Val::Px(60.0),
                    justify_content: JustifyContent::Center,
                    align_items:     AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.1, 0.4, 0.1, 0.92)),
                ..default()
            },
            BtnCastle,
        ))
        .with_children(|b| {
            b.spawn(TextBundle::from_section(
                "Enrocar",
                TextStyle { font, font_size: 28.0, color: Color::rgb(0.95, 0.95, 0.95) },
            ));
        });
    });
}

fn execute_pending_castle(
    btn_q:              Query<&Interaction, (Changed<Interaction>, With<BtnCastle>)>,
    mut pending:        ResMut<CastlingPending>,
    mut pieces_query:   Query<(Entity, &mut Piece)>,
    mut turn:           ResMut<PlayerTurn>,
    mut castling_state: ResMut<CastlingState>,
    mut en_passant:       ResMut<EnPassantTarget>,
    mut history:          ResMut<GameHistory>,
    mut draw_state:       ResMut<DrawTracking>,
    mut status_event:     EventWriter<GameStatusEvent>,
    mut reset_event:      EventWriter<ResetSelectedEvent>,
    root_q:               Query<Entity, With<CastleConfirmRoot>>,
    mut commands:         Commands,
) {
    let pressed = btn_q.iter().any(|i| *i == Interaction::Pressed);
    if !pressed { return; }

    let (king_ent, side) = match (pending.king_entity, pending.side) {
        (Some(e), Some(s)) => (e, s),
        _ => return,
    };

    // Snapshot board state before any mutations
    let pieces_entity_vec: Vec<(Entity, Piece)> = pieces_query.iter().map(|(e, p)| (e, *p)).collect();

    let king_snap = match pieces_entity_vec.iter().find(|(e, _)| *e == king_ent).map(|(_, p)| *p) {
        Some(k) => k,
        None => return,
    };

    let dest_file:      u8 = match side { CastleSide::Kingside => 6, CastleSide::Queenside => 2 };
    let rook_from_file: u8 = match side { CastleSide::Kingside => 7, CastleSide::Queenside => 0 };
    let rook_to_file:   u8 = match side { CastleSide::Kingside => 5, CastleSide::Queenside => 3 };

    // Record move in history (before mutations)
    {
        let pieces_vec: Vec<Piece> = pieces_entity_vec.iter().map(|(_, p)| *p).collect();
        let from_eng = EngineSquare(king_snap.x * 8 + king_snap.y);
        let to_eng   = EngineSquare(king_snap.x * 8 + dest_file);
        let pre_fen  = build_fen_ep(&pieces_vec, turn.0, &castling_state, en_passant.0);
        if let Ok(pre_pos) = chess_engine::Position::from_fen(&pre_fen) {
            if let Some(eng_mv) = find_engine_move(&pre_pos, from_eng, to_eng) {
                history.moves.push(eng_mv);
            }
        }
    }

    // Move king
    if let Ok((_, mut king_p)) = pieces_query.get_mut(king_ent) {
        king_p.y = dest_file;
    }

    // Teleport rook
    if let Some((rook_ent, _)) = pieces_entity_vec.iter()
        .find(|(_, p)| p.x == king_snap.x && p.y == rook_from_file
            && p.piece_type == PieceType::Rook && p.color == king_snap.color)
    {
        if let Ok((_, mut rook_p)) = pieces_query.get_mut(*rook_ent) {
            rook_p.y = rook_to_file;
        }
    }

    // Update castling rights
    match king_snap.color {
        PieceColor::White => { castling_state.white_kingside  = false; castling_state.white_queenside  = false; }
        PieceColor::Black => { castling_state.black_kingside  = false; castling_state.black_queenside  = false; }
    }

    en_passant.0 = None;
    draw_state.halfmove_clock += 1; // castling: no capture, no pawn move

    turn.change();

    // Check / checkmate / stalemate detection
    let all_pieces: Vec<Piece> = pieces_query.iter().map(|(_, p)| *p).collect();

    let pos_key = position_key(&all_pieces, turn.0, &castling_state, None);
    let is_draw = draw_state.record_position(pos_key);

    let pawns_valid = all_pieces.iter().all(|p| {
        p.piece_type != PieceType::Pawn || (p.x > 0 && p.x < 7)
    });
    if pawns_valid {
        let fen = build_fen_ep(&all_pieces, turn.0, &castling_state, None);
        if let Ok(pos) = Position::from_fen(&fen) {
            if pos.is_checkmate() {
                let winner = match turn.0 {
                    PieceColor::White => PieceColor::Black,
                    PieceColor::Black => PieceColor::White,
                };
                status_event.send(GameStatusEvent(GameStatus::Checkmate { winner }));
            } else if pos.is_stalemate() || is_draw {
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

    // Cleanup
    *pending = CastlingPending::default();
    for e in root_q.iter() { commands.entity(e).despawn_recursive(); }
    reset_event.send(ResetSelectedEvent);
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
            .init_resource::<CastlingPending>()
            .init_resource::<EnPassantTarget>()
            .init_resource::<DrawTracking>()
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
                    show_castling_button,
                    execute_pending_castle,
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

    #[test]
    fn en_passant_capture_is_highlighted() {
        // White pawn on e5 (rank 4, file 4). Black just played d7-d5 (rank 4, file 3).
        // En passant target: d6 = rank 5, file 3. FEN encodes this as "d6".
        let fen = "4k3/ppp1pppp/8/3pP3/8/8/PPPP1PPP/4K3 w - d6 0 1";
        let squares = legal_squares_for(fen, 4, 4); // White pawn at e5
        assert!(
            squares.contains(&(5, 3)),
            "White pawn must be able to capture en passant to d6 (rank 5, file 3), got: {:?}",
            squares
        );
    }
}
