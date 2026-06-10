use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use crate::state::AppState;

fn spawn_piece(
    commands: &mut Commands,
    material: Handle<StandardMaterial>,
    piece_color: PieceColor,
    piece_type: PieceType,
    meshes: Vec<Handle<Mesh>>,
    offsets: Vec<Vec3>,
    scale: Vec3,
    position: (u8, u8),
) {
    commands
        .spawn((
            PbrBundle {
                transform: Transform::from_translation(Vec3::new(
                    position.0 as f32,
                    0.0,
                    position.1 as f32,
                )),
                ..default()
            },
            Piece {
                color: piece_color,
                piece_type,
                x: position.0,
                y: position.1,
            },
        ))
        .with_children(|parent| {
            for (mesh, offset) in meshes.into_iter().zip(offsets.into_iter()) {
                parent.spawn((
                    PbrBundle {
                        mesh,
                        material: material.clone(),
                        transform: Transform {
                            translation: offset,
                            scale,
                            ..default()
                        },
                        ..default()
                    },
                    PickableBundle::default(),
                ));
            }
        });
}

fn create_pieces(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let king_handle: Handle<Mesh> =
        asset_server.load("models/chess_kit/pieces.glb#Mesh0/Primitive0");
    let king_cross_handle: Handle<Mesh> =
        asset_server.load("models/chess_kit/pieces.glb#Mesh1/Primitive0");
    let pawn_handle: Handle<Mesh> =
        asset_server.load("models/chess_kit/pieces.glb#Mesh2/Primitive0");
    let knight_1_handle: Handle<Mesh> =
        asset_server.load("models/chess_kit/pieces.glb#Mesh3/Primitive0");
    let knight_2_handle: Handle<Mesh> =
        asset_server.load("models/chess_kit/pieces.glb#Mesh4/Primitive0");
    let rook_handle: Handle<Mesh> =
        asset_server.load("models/chess_kit/pieces.glb#Mesh5/Primitive0");
    let bishop_handle: Handle<Mesh> =
        asset_server.load("models/chess_kit/pieces.glb#Mesh6/Primitive0");
    let queen_handle: Handle<Mesh> =
        asset_server.load("models/chess_kit/pieces.glb#Mesh7/Primitive0");

    let white = materials.add(Color::rgb(1.0, 0.8, 0.8));
    let black = materials.add(Color::rgb(0.0, 0.2, 0.2));
    let s = Vec3::splat(0.2);

    let sp = |commands: &mut Commands,
              mat: Handle<StandardMaterial>,
              color: PieceColor,
              pt: PieceType,
              ms: Vec<Handle<Mesh>>,
              offs: Vec<Vec3>,
              pos: (u8, u8)| {
        spawn_piece(commands, mat, color, pt, ms, offs, s, pos);
    };

    // White pieces
    sp(&mut commands, white.clone(), PieceColor::White, PieceType::Rook,
        vec![rook_handle.clone()], vec![Vec3::new(-0.1, 0., 1.8)], (0, 0));
    sp(&mut commands, white.clone(), PieceColor::White, PieceType::Knight,
        vec![knight_1_handle.clone(), knight_2_handle.clone()],
        vec![Vec3::new(-0.2, 0., 0.9); 2], (0, 1));
    sp(&mut commands, white.clone(), PieceColor::White, PieceType::Bishop,
        vec![bishop_handle.clone()], vec![Vec3::new(-0.1, 0., 0.)], (0, 2));
    sp(&mut commands, white.clone(), PieceColor::White, PieceType::Queen,
        vec![queen_handle.clone()], vec![Vec3::new(-0.2, 0., -0.95)], (0, 3));
    sp(&mut commands, white.clone(), PieceColor::White, PieceType::King,
        vec![king_handle.clone(), king_cross_handle.clone()],
        vec![Vec3::new(-0.2, 0., -1.9); 2], (0, 4));
    sp(&mut commands, white.clone(), PieceColor::White, PieceType::Bishop,
        vec![bishop_handle.clone()], vec![Vec3::new(-0.1, 0., 0.)], (0, 5));
    sp(&mut commands, white.clone(), PieceColor::White, PieceType::Knight,
        vec![knight_1_handle.clone(), knight_2_handle.clone()],
        vec![Vec3::new(-0.2, 0., 0.9); 2], (0, 6));
    sp(&mut commands, white.clone(), PieceColor::White, PieceType::Rook,
        vec![rook_handle.clone()], vec![Vec3::new(-0.1, 0., 1.8)], (0, 7));
    for i in 0u8..8 {
        sp(&mut commands, white.clone(), PieceColor::White, PieceType::Pawn,
            vec![pawn_handle.clone()], vec![Vec3::new(-0.2, 0., 2.6)], (1, i));
    }

    // Black pieces
    sp(&mut commands, black.clone(), PieceColor::Black, PieceType::Rook,
        vec![rook_handle.clone()], vec![Vec3::new(-0.1, 0., 1.8)], (7, 0));
    sp(&mut commands, black.clone(), PieceColor::Black, PieceType::Knight,
        vec![knight_1_handle.clone(), knight_2_handle.clone()],
        vec![Vec3::new(-0.2, 0., 0.9); 2], (7, 1));
    sp(&mut commands, black.clone(), PieceColor::Black, PieceType::Bishop,
        vec![bishop_handle.clone()], vec![Vec3::new(-0.1, 0., 0.)], (7, 2));
    sp(&mut commands, black.clone(), PieceColor::Black, PieceType::Queen,
        vec![queen_handle.clone()], vec![Vec3::new(-0.2, 0., -0.95)], (7, 3));
    sp(&mut commands, black.clone(), PieceColor::Black, PieceType::King,
        vec![king_handle.clone(), king_cross_handle.clone()],
        vec![Vec3::new(-0.2, 0., -1.9); 2], (7, 4));
    sp(&mut commands, black.clone(), PieceColor::Black, PieceType::Bishop,
        vec![bishop_handle.clone()], vec![Vec3::new(-0.1, 0., 0.)], (7, 5));
    sp(&mut commands, black.clone(), PieceColor::Black, PieceType::Knight,
        vec![knight_1_handle.clone(), knight_2_handle.clone()],
        vec![Vec3::new(-0.2, 0., 0.9); 2], (7, 6));
    sp(&mut commands, black.clone(), PieceColor::Black, PieceType::Rook,
        vec![rook_handle.clone()], vec![Vec3::new(-0.1, 0., 1.8)], (7, 7));
    for i in 0u8..8 {
        sp(&mut commands, black.clone(), PieceColor::Black, PieceType::Pawn,
            vec![pawn_handle.clone()], vec![Vec3::new(-0.2, 0., 2.6)], (6, i));
    }
}

#[derive(Clone, Copy, PartialEq, Component)]
pub enum PieceColor {
    White,
    Black,
}

#[derive(Clone, Copy, PartialEq, Debug, Component)]
pub enum PieceType {
    King,
    Queen,
    Bishop,
    Knight,
    Rook,
    Pawn,
}

#[derive(Clone, Copy, Component)]
pub struct Piece {
    pub color: PieceColor,
    pub piece_type: PieceType,
    pub x: u8,
    pub y: u8,
}

impl Piece {
    pub fn is_move_valid(&self, new_position: (u8, u8), pieces: Vec<Piece>) -> bool {
        if color_of_square(new_position, &pieces) == Some(self.color) {
            return false;
        }
        match self.piece_type {
            PieceType::King => {
                ((self.x as i8 - new_position.0 as i8).abs() == 1 && self.y == new_position.1)
                    || ((self.y as i8 - new_position.1 as i8).abs() == 1
                        && self.x == new_position.0)
                    || ((self.x as i8 - new_position.0 as i8).abs() == 1
                        && (self.y as i8 - new_position.1 as i8).abs() == 1)
            }
            PieceType::Queen => {
                is_path_empty((self.x, self.y), new_position, &pieces)
                    && ((self.x as i8 - new_position.0 as i8).abs()
                        == (self.y as i8 - new_position.1 as i8).abs()
                        || ((self.x == new_position.0 && self.y != new_position.1)
                            || (self.y == new_position.1 && self.x != new_position.0)))
            }
            PieceType::Bishop => {
                is_path_empty((self.x, self.y), new_position, &pieces)
                    && (self.x as i8 - new_position.0 as i8).abs()
                        == (self.y as i8 - new_position.1 as i8).abs()
            }
            PieceType::Knight => {
                ((self.x as i8 - new_position.0 as i8).abs() == 2
                    && (self.y as i8 - new_position.1 as i8).abs() == 1)
                    || ((self.x as i8 - new_position.0 as i8).abs() == 1
                        && (self.y as i8 - new_position.1 as i8).abs() == 2)
            }
            PieceType::Rook => {
                is_path_empty((self.x, self.y), new_position, &pieces)
                    && ((self.x == new_position.0 && self.y != new_position.1)
                        || (self.y == new_position.1 && self.x != new_position.0))
            }
            PieceType::Pawn => {
                if self.color == PieceColor::White {
                    (new_position.0 as i8 - self.x as i8 == 1
                        && self.y == new_position.1
                        && color_of_square(new_position, &pieces).is_none())
                        || (self.x == 1
                            && new_position.0 as i8 - self.x as i8 == 2
                            && self.y == new_position.1
                            && is_path_empty((self.x, self.y), new_position, &pieces)
                            && color_of_square(new_position, &pieces).is_none())
                        || (new_position.0 as i8 - self.x as i8 == 1
                            && (self.y as i8 - new_position.1 as i8).abs() == 1
                            && color_of_square(new_position, &pieces) == Some(PieceColor::Black))
                } else {
                    (new_position.0 as i8 - self.x as i8 == -1
                        && self.y == new_position.1
                        && color_of_square(new_position, &pieces).is_none())
                        || (self.x == 6
                            && new_position.0 as i8 - self.x as i8 == -2
                            && self.y == new_position.1
                            && is_path_empty((self.x, self.y), new_position, &pieces)
                            && color_of_square(new_position, &pieces).is_none())
                        || (new_position.0 as i8 - self.x as i8 == -1
                            && (self.y as i8 - new_position.1 as i8).abs() == 1
                            && color_of_square(new_position, &pieces) == Some(PieceColor::White))
                }
            }
        }
    }
}

fn move_pieces(time: Res<Time>, mut query: Query<(&mut Transform, &Piece)>) {
    for (mut transform, piece) in query.iter_mut() {
        let target = Vec3::new(piece.x as f32, 0.0, piece.y as f32);
        let diff = target - transform.translation;
        let dist = diff.length();
        if dist > 0.001 {
            let step = (time.delta_seconds() * 15.0).min(dist);
            transform.translation += diff / dist * step;
        } else {
            transform.translation = target;
        }
    }
}

fn despawn_pieces(mut commands: Commands, query: Query<Entity, With<Piece>>) {
    for e in &query {
        commands.entity(e).despawn_recursive();
    }
}

fn color_of_square(pos: (u8, u8), pieces: &[Piece]) -> Option<PieceColor> {
    pieces
        .iter()
        .find(|p| p.x == pos.0 && p.y == pos.1)
        .map(|p| p.color)
}

fn is_path_empty(begin: (u8, u8), end: (u8, u8), pieces: &[Piece]) -> bool {
    if begin.0 == end.0 {
        if pieces.iter().any(|p| {
            p.x == begin.0
                && ((p.y > begin.1 && p.y < end.1) || (p.y > end.1 && p.y < begin.1))
        }) {
            return false;
        }
    }
    if begin.1 == end.1 {
        if pieces.iter().any(|p| {
            p.y == begin.1
                && ((p.x > begin.0 && p.x < end.0) || (p.x > end.0 && p.x < begin.0))
        }) {
            return false;
        }
    }
    let x_diff = (begin.0 as i8 - end.0 as i8).abs();
    let y_diff = (begin.1 as i8 - end.1 as i8).abs();
    if x_diff == y_diff {
        for i in 1..x_diff {
            let pos = if begin.0 < end.0 && begin.1 < end.1 {
                (begin.0 + i as u8, begin.1 + i as u8)
            } else if begin.0 < end.0 && begin.1 > end.1 {
                (begin.0 + i as u8, begin.1 - i as u8)
            } else if begin.0 > end.0 && begin.1 < end.1 {
                (begin.0 - i as u8, begin.1 + i as u8)
            } else {
                (begin.0 - i as u8, begin.1 - i as u8)
            };
            if color_of_square(pos, pieces).is_some() {
                return false;
            }
        }
    }
    true
}

pub struct PiecesPlugin;

impl Plugin for PiecesPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Playing), create_pieces)
            .add_systems(OnExit(AppState::Playing), despawn_pieces)
            .add_systems(Update, move_pieces.run_if(in_state(AppState::Playing)));
    }
}
