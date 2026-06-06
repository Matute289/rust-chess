use bevy::{app::AppExit, prelude::*};
use bevy_mod_picking::prelude::*;
use crate::pieces::{Piece, PieceColor, PieceType};

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

#[derive(Resource)]
struct SquareMaterials {
    highlight_color: Handle<StandardMaterial>,
    selected_color: Handle<StandardMaterial>,
    black_color: Handle<StandardMaterial>,
    white_color: Handle<StandardMaterial>,
}

impl FromWorld for SquareMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        SquareMaterials {
            highlight_color: materials.add(Color::rgb(0.8, 0.3, 0.3)),
            selected_color: materials.add(Color::rgb(0.9, 0.1, 0.1)),
            black_color: materials.add(Color::rgb(0.0, 0.1, 0.1)),
            white_color: materials.add(Color::rgb(1.0, 0.9, 0.9)),
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
    materials: Res<SquareMaterials>,
    mut query: Query<(Entity, &Square, &mut Handle<StandardMaterial>, Option<&PickingInteraction>)>,
) {
    for (entity, square, mut material, interaction) in query.iter_mut() {
        *material = match interaction {
            Some(PickingInteraction::Hovered | PickingInteraction::Pressed) => {
                materials.highlight_color.clone()
            }
            _ if Some(entity) == selected_square.entity => materials.selected_color.clone(),
            _ if square.is_white() => materials.white_color.clone(),
            _ => materials.black_color.clone(),
        };
    }
}

fn select_square(
    mut click_events: EventReader<Pointer<Click>>,
    squares_query: Query<&Square>,
    mut selected_square: ResMut<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
) {
    for event in click_events.read() {
        if squares_query.get(event.target).is_ok() {
            selected_square.entity = Some(event.target);
        } else {
            selected_square.entity = None;
            selected_piece.entity = None;
        }
    }
}

fn select_piece(
    selected_square: Res<SelectedSquare>,
    mut selected_piece: ResMut<SelectedPiece>,
    turn: Res<PlayerTurn>,
    squares_query: Query<&Square>,
    pieces_query: Query<(Entity, &Piece)>,
) {
    if !selected_square.is_changed() {
        return;
    }
    let square_entity = match selected_square.entity {
        Some(e) => e,
        None => return,
    };
    let square = match squares_query.get(square_entity) {
        Ok(s) => s,
        Err(_) => return,
    };

    if selected_piece.entity.is_none() {
        for (piece_entity, piece) in pieces_query.iter() {
            if piece.x == square.x && piece.y == square.y && piece.color == turn.0 {
                selected_piece.entity = Some(piece_entity);
                break;
            }
        }
    }
}

fn move_piece(
    mut commands: Commands,
    selected_square: Res<SelectedSquare>,
    selected_piece: Res<SelectedPiece>,
    mut turn: ResMut<PlayerTurn>,
    squares_query: Query<&Square>,
    mut pieces_query: Query<(Entity, &mut Piece)>,
    mut reset_event: EventWriter<ResetSelectedEvent>,
) {
    if !selected_square.is_changed() {
        return;
    }
    let square_entity = match selected_square.entity {
        Some(e) => e,
        None => return,
    };
    let square = match squares_query.get(square_entity) {
        Ok(s) => s,
        Err(_) => return,
    };
    let selected_piece_entity = match selected_piece.entity {
        Some(e) => e,
        None => return,
    };

    let pieces_vec: Vec<Piece> = pieces_query.iter().map(|(_, p)| *p).collect();
    let pieces_entity_vec: Vec<(Entity, Piece)> =
        pieces_query.iter().map(|(e, p)| (e, *p)).collect();

    if let Ok((_, mut piece)) = pieces_query.get_mut(selected_piece_entity) {
        if piece.is_move_valid((square.x, square.y), pieces_vec) {
            for (other_entity, other_piece) in &pieces_entity_vec {
                if other_piece.x == square.x
                    && other_piece.y == square.y
                    && other_piece.color != piece.color
                {
                    commands.entity(*other_entity).insert(Taken);
                }
            }
            piece.x = square.x;
            piece.y = square.y;
            turn.change();
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
struct Taken;

fn despawn_taken_pieces(
    mut commands: Commands,
    mut app_exit: EventWriter<AppExit>,
    query: Query<(Entity, &Piece), With<Taken>>,
) {
    for (entity, piece) in query.iter() {
        if piece.piece_type == PieceType::King {
            println!(
                "{} ganaron! Gracias por jugar!",
                match piece.color {
                    PieceColor::White => "Negras",
                    PieceColor::Black => "Blancas",
                }
            );
            app_exit.send(AppExit::Success);
        }
        commands.entity(entity).despawn_recursive();
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
    fn change(&mut self) {
        self.0 = match self.0 {
            PieceColor::White => PieceColor::Black,
            PieceColor::Black => PieceColor::White,
        }
    }
}

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedSquare>()
            .init_resource::<SelectedPiece>()
            .init_resource::<PlayerTurn>()
            .init_resource::<SquareMaterials>()
            .add_event::<ResetSelectedEvent>()
            .add_systems(Startup, create_board)
            .add_systems(
                Update,
                (
                    select_square,
                    color_squares,
                    select_piece,
                    move_piece,
                    despawn_taken_pieces,
                    reset_selected,
                ),
            );
    }
}
