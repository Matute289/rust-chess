use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use crate::ai::build_fen;
use crate::captured::{CapturedPieces, PromotionPending};
use crate::pieces::{Piece, PieceColor, PieceType};
use crate::state::AppState;
use chess_engine::Position;

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
    Check,
    Checkmate { winner: PieceColor },
    Stalemate,
}

#[derive(Component)]
pub struct BadMoveFlash(pub Timer);

#[derive(Resource)]
struct SquareMaterials {
    highlight_color: Handle<StandardMaterial>,  // hover = blue
    selected_color:  Handle<StandardMaterial>,  // selected = green
    valid_color:     Handle<StandardMaterial>,  // valid move dest = lighter green
    bad_move_color:  Handle<StandardMaterial>,  // invalid move flash = red
    black_color:     Handle<StandardMaterial>,
    white_color:     Handle<StandardMaterial>,
}

impl FromWorld for SquareMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        SquareMaterials {
            highlight_color: materials.add(Color::rgb(0.1, 0.3, 0.8)),   // blue
            selected_color:  materials.add(Color::rgb(0.1, 0.7, 0.2)),   // green
            valid_color:     materials.add(Color::rgb(0.2, 0.6, 0.15)),  // lighter green
            bad_move_color:  materials.add(Color::rgb(0.8, 0.1, 0.1)),   // red
            black_color:     materials.add(Color::rgb(0.0, 0.1, 0.1)),
            white_color:     materials.add(Color::rgb(1.0, 0.9, 0.9)),
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
                *material = materials.bad_move_color.clone();
                continue;
            }
            commands.entity(entity).remove::<BadMoveFlash>();
        }

        let is_valid = valid_moves.0.contains(&(square.x, square.y));

        *material = match interaction {
            Some(PickingInteraction::Hovered | PickingInteraction::Pressed) => {
                materials.highlight_color.clone()
            }
            _ if Some(entity) == selected_square.entity => materials.selected_color.clone(),
            _ if is_valid => materials.valid_color.clone(),
            _ if square.is_white() => materials.white_color.clone(),
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
    squares_query:      Query<&Square>,
    pieces_query:       Query<(Entity, &Piece)>,
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
                selected_piece.entity = Some(piece_entity);
                let pieces_vec: Vec<Piece> = pieces_query.iter().map(|(_, p)| *p).collect();
                valid_moves.0 = (0u8..8)
                    .flat_map(|x| (0u8..8).map(move |y| (x, y)))
                    .filter(|&pos| piece.is_move_valid(pos, pieces_vec.clone()))
                    .collect();
                break;
            }
        }
    }
}

fn move_piece(
    mut commands:       Commands,
    selected_square:    Res<SelectedSquare>,
    selected_piece:     Res<SelectedPiece>,
    mut turn:           ResMut<PlayerTurn>,
    mut castling_state: ResMut<CastlingState>,
    mut captured:       ResMut<CapturedPieces>,
    mut promotion:      ResMut<PromotionPending>,
    mut valid_moves:    ResMut<ValidMoveSquares>,
    squares_query:      Query<(Entity, &Square)>,
    mut pieces_query:   Query<(Entity, &mut Piece)>,
    mut reset_event:    EventWriter<ResetSelectedEvent>,
    mut status_event:   EventWriter<GameStatusEvent>,
) {
    if !selected_square.is_changed() { return; }
    if promotion.is_pending() { return; }

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
        if piece.is_move_valid((square_x, square_y), pieces_vec) {
            // Capture
            for (other_entity, other_piece) in &pieces_entity_vec {
                if other_piece.x == square_x && other_piece.y == square_y && other_piece.color != piece.color {
                    captured.add(other_piece);
                    commands.entity(*other_entity).insert(Taken);
                }
            }

            let origin_y = piece.y;
            piece.x = square_x;
            piece.y = square_y;
            valid_moves.0.clear();

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
            let all_pieces: Vec<Piece> = pieces_query.iter().map(|(_, p)| *p).collect();
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
                }
            }
        } else {
            // Bad move: flash destination square red for 0.5s
            if let Some((sq_entity, _)) = squares_query.iter().find(|(_, s)| s.x == square_x && s.y == square_y) {
                commands.entity(sq_entity).insert(BadMoveFlash(Timer::from_seconds(0.5, TimerMode::Once)));
            }
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
) {
    for _ in events.read() {
        selected_square.entity = None;
        selected_piece.entity = None;
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

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<SelectedSquare>()
            .init_resource::<SelectedPiece>()
            .init_resource::<PlayerTurn>()
            .init_resource::<CastlingState>()
            .init_resource::<ValidMoveSquares>()
            .init_resource::<SquareMaterials>()
            .add_event::<ResetSelectedEvent>()
            .add_event::<GameStatusEvent>()
            .add_systems(Startup, create_board)
            .add_systems(OnEnter(AppState::Playing), reset_board_state)
            .add_systems(
                Update,
                (
                    select_square,
                    color_squares,
                    select_piece,
                    move_piece,
                    despawn_taken_pieces,
                    reset_selected,
                ).run_if(in_state(AppState::Playing)),
            );
    }
}
