use bevy::prelude::*;
use crate::pieces::{Piece, PieceColor, PieceType};
use crate::state::AppState;
use crate::board::{CastlingState, GameStatus, GameStatusEvent, PlayerTurn};
use crate::ai::build_fen;
use chess_engine::Position;

// ─── Resources ───────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct CapturedPieces {
    pub white_captured: Vec<PieceType>, // White pieces captured by Black
    pub black_captured: Vec<PieceType>, // Black pieces captured by White
}

impl CapturedPieces {
    pub fn add(&mut self, piece: &Piece) {
        match piece.color {
            PieceColor::White => self.white_captured.push(piece.piece_type),
            PieceColor::Black => self.black_captured.push(piece.piece_type),
        }
    }

    pub fn remove_first(&mut self, color: PieceColor, pt: PieceType) -> bool {
        let list = match color {
            PieceColor::White => &mut self.white_captured,
            PieceColor::Black => &mut self.black_captured,
        };
        if let Some(pos) = list.iter().position(|&t| t == pt) {
            list.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn available_for_promotion(&self, promoting_color: PieceColor) -> &[PieceType] {
        // Offer the promoting player's OWN captured pieces to recover
        match promoting_color {
            PieceColor::White => &self.white_captured,  // White's own pieces captured by Black
            PieceColor::Black => &self.black_captured,  // Black's own pieces captured by White
        }
    }
}

// ─── Promotion state ─────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct PromotionPending {
    pub pawn_entity: Option<Entity>,
    pub color: Option<PieceColor>,
}

impl PromotionPending {
    pub fn is_pending(&self) -> bool { self.pawn_entity.is_some() }
}

// ─── Components ──────────────────────────────────────────────────────────────

#[derive(Component)] pub struct CapturedPieceDisplay;
#[derive(Component)] struct PromotionOverlayRoot;
#[derive(Component)] pub struct PromotionBtn(pub PieceType);

// ─── Piece name helper ────────────────────────────────────────────────────────

fn piece_name(pt: PieceType) -> &'static str {
    match pt {
        PieceType::Queen  => "Reina",
        PieceType::Rook   => "Torre",
        PieceType::Bishop => "Alfil",
        PieceType::Knight => "Caballo",
        PieceType::Pawn   => "Peón",
        PieceType::King   => "Rey",
    }
}

fn piece_meshes_and_offsets(asset_server: &AssetServer, pt: PieceType) -> (Vec<Handle<Mesh>>, Vec<Vec3>) {
    match pt {
        PieceType::King => (
            vec![
                asset_server.load("models/chess_kit/pieces.glb#Mesh0/Primitive0"),
                asset_server.load("models/chess_kit/pieces.glb#Mesh1/Primitive0"),
            ],
            vec![Vec3::new(-0.2, 0., -1.9); 2],
        ),
        PieceType::Queen => (
            vec![asset_server.load("models/chess_kit/pieces.glb#Mesh7/Primitive0")],
            vec![Vec3::new(-0.2, 0., -0.95)],
        ),
        PieceType::Rook => (
            vec![asset_server.load("models/chess_kit/pieces.glb#Mesh5/Primitive0")],
            vec![Vec3::new(-0.1, 0., 1.8)],
        ),
        PieceType::Bishop => (
            vec![asset_server.load("models/chess_kit/pieces.glb#Mesh6/Primitive0")],
            vec![Vec3::new(-0.1, 0., 0.)],
        ),
        PieceType::Knight => (
            vec![
                asset_server.load("models/chess_kit/pieces.glb#Mesh3/Primitive0"),
                asset_server.load("models/chess_kit/pieces.glb#Mesh4/Primitive0"),
            ],
            vec![Vec3::new(-0.2, 0., 0.9); 2],
        ),
        PieceType::Pawn => (
            vec![asset_server.load("models/chess_kit/pieces.glb#Mesh2/Primitive0")],
            vec![Vec3::new(-0.2, 0., 2.6)],
        ),
    }
}

// ─── Systems ─────────────────────────────────────────────────────────────────

pub fn reset_captured(
    mut captured: ResMut<CapturedPieces>,
    mut promotion: ResMut<PromotionPending>,
) {
    *captured = CapturedPieces::default();
    *promotion = PromotionPending::default();
}

fn despawn_panels(
    mut commands: Commands,
    display_q: Query<Entity, With<CapturedPieceDisplay>>,
    overlay: Query<Entity, With<PromotionOverlayRoot>>,
) {
    for e in display_q.iter().chain(overlay.iter()) {
        commands.entity(e).despawn_recursive();
    }
}

fn update_captured_3d(
    mut commands: Commands,
    captured: Res<CapturedPieces>,
    display_q: Query<Entity, With<CapturedPieceDisplay>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !captured.is_changed() { return; }
    for e in &display_q { commands.entity(e).despawn_recursive(); }

    let white_mat = materials.add(Color::rgb(1.0, 0.8, 0.8));
    let black_mat = materials.add(Color::rgb(0.0, 0.2, 0.2));
    let scale = Vec3::splat(0.15);

    // White pieces captured by Black — shown on left side (x = -2.0)
    for (i, &pt) in captured.white_captured.iter().enumerate() {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        let pos = Vec3::new(-2.0 + col * 0.8, 0.0, row * 1.2);
        let (meshes, offsets) = piece_meshes_and_offsets(&asset_server, pt);
        commands.spawn((
            PbrBundle { transform: Transform::from_translation(pos), ..default() },
            CapturedPieceDisplay,
        )).with_children(|p| {
            for (mesh, offset) in meshes.into_iter().zip(offsets.into_iter()) {
                p.spawn(PbrBundle {
                    mesh,
                    material: white_mat.clone(),
                    transform: Transform { translation: offset, scale, ..default() },
                    ..default()
                });
            }
        });
    }

    // Black pieces captured by White — shown on right side (x = 8.5)
    for (i, &pt) in captured.black_captured.iter().enumerate() {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        let pos = Vec3::new(8.5 + col * 0.8, 0.0, row * 1.2);
        let (meshes, offsets) = piece_meshes_and_offsets(&asset_server, pt);
        commands.spawn((
            PbrBundle { transform: Transform::from_translation(pos), ..default() },
            CapturedPieceDisplay,
        )).with_children(|p| {
            for (mesh, offset) in meshes.into_iter().zip(offsets.into_iter()) {
                p.spawn(PbrBundle {
                    mesh,
                    material: black_mat.clone(),
                    transform: Transform { translation: offset, scale, ..default() },
                    ..default()
                });
            }
        });
    }
}

pub fn show_promotion_overlay(
    mut commands: Commands,
    promotion: Res<PromotionPending>,
    captured: Res<CapturedPieces>,
    overlay_q: Query<Entity, With<PromotionOverlayRoot>>,
    asset_server: Res<AssetServer>,
) {
    if !promotion.is_changed() { return; }
    for e in &overlay_q { commands.entity(e).despawn_recursive(); }
    if !promotion.is_pending() { return; }

    let color = match promotion.color { Some(c) => c, None => return };
    let available = captured.available_for_promotion(color);
    let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.78)),
            ..default()
        },
        PromotionOverlayRoot,
    )).with_children(|root| {
        root.spawn(TextBundle::from_section(
            "¡Coronación! Elegí una pieza capturada:",
            TextStyle { font: font.clone(), font_size: 34.0, color: Color::rgb(1.0, 0.88, 0.2) },
        ));
        root.spawn(NodeBundle { style: Style { height: Val::Px(10.0), ..default() }, ..default() });

        if available.is_empty() {
            root.spawn(TextBundle::from_section(
                "Sin piezas capturadas — se corona en Reina automáticamente.",
                TextStyle { font: font.clone(), font_size: 22.0, color: Color::rgb(0.8, 0.8, 0.8) },
            ));
            spawn_promo_btn(root, font.clone(), PieceType::Queen);
        } else {
            let mut shown: Vec<PieceType> = Vec::new();
            for &pt in available {
                if shown.contains(&pt) { continue; }
                shown.push(pt);
                spawn_promo_btn(root, font.clone(), pt);
            }
        }
    });
}

fn spawn_promo_btn(parent: &mut ChildBuilder, font: Handle<Font>, pt: PieceType) {
    parent.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(280.0),
                height: Val::Px(60.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.18, 0.18, 0.42, 0.92)),
            ..default()
        },
        PromotionBtn(pt),
    )).with_children(|p| {
        p.spawn(TextBundle::from_section(
            piece_name(pt),
            TextStyle { font, font_size: 28.0, color: Color::rgb(0.92, 0.92, 0.92) },
        ));
    });
}

pub fn handle_promotion_choice(
    q:             Query<(&Interaction, &PromotionBtn), Changed<Interaction>>,
    mut commands:  Commands,
    mut promotion: ResMut<PromotionPending>,
    mut captured:  ResMut<CapturedPieces>,
    mut pieces_q:  Query<(Entity, &mut Piece)>,
    overlay_q:     Query<Entity, With<PromotionOverlayRoot>>,
    mut turn:      ResMut<PlayerTurn>,
    castling:      Res<CastlingState>,
    mut status_ev: EventWriter<GameStatusEvent>,
    asset_server:  Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (interaction, btn) in &q {
        if *interaction != Interaction::Pressed { continue; }

        let pawn_entity = match promotion.pawn_entity { Some(e) => e, None => continue };
        let color       = match promotion.color       { Some(c) => c, None => continue };

        // Add the spent pawn to own captured pile (pawn is "used up" in promotion)
        captured.add(&Piece { color, piece_type: PieceType::Pawn, x: 0, y: 0 });

        // Apply promotion + update 3D mesh
        if let Ok((_, mut piece)) = pieces_q.get_mut(pawn_entity) {
            piece.piece_type = btn.0;
        }
        commands.entity(pawn_entity).despawn_descendants();
        let (meshes, offsets) = piece_meshes_and_offsets(&asset_server, btn.0);
        let mat = materials.add(match color {
            PieceColor::White => Color::rgb(1.0, 0.8, 0.8),
            PieceColor::Black => Color::rgb(0.0, 0.2, 0.2),
        });
        let scale = Vec3::splat(0.2);
        commands.entity(pawn_entity).with_children(|parent| {
            for (mesh, offset) in meshes.into_iter().zip(offsets.into_iter()) {
                parent.spawn((
                    PbrBundle {
                        mesh,
                        material: mat.clone(),
                        transform: Transform { translation: offset, scale, ..default() },
                        ..default()
                    },
                    bevy_mod_picking::prelude::PickableBundle::default(),
                ));
            }
        });

        // Remove the recovered piece from own captured pile
        captured.remove_first(color, btn.0);

        // Clear promotion state and despawn overlay
        *promotion = PromotionPending::default();
        for e in &overlay_q { commands.entity(e).despawn_recursive(); }

        // Advance turn
        turn.change();

        // Snapshot current board state for check detection
        let all: Vec<crate::pieces::Piece> = pieces_q.iter().map(|(_, p)| *p).collect();
        let fen = build_fen(&all, turn.0, &castling);
        if let Ok(pos) = Position::from_fen(&fen) {
            if pos.is_checkmate() {
                let winner = match turn.0 {
                    crate::pieces::PieceColor::White => crate::pieces::PieceColor::Black,
                    crate::pieces::PieceColor::Black => crate::pieces::PieceColor::White,
                };
                status_ev.send(GameStatusEvent(GameStatus::Checkmate { winner }));
            } else if pos.is_stalemate() {
                status_ev.send(GameStatusEvent(GameStatus::Stalemate));
            } else if pos.is_in_check() {
                status_ev.send(GameStatusEvent(GameStatus::Check));
            } else {
                status_ev.send(GameStatusEvent(GameStatus::Ok));
            }
        } else {
            status_ev.send(GameStatusEvent(GameStatus::Ok));
        }

        return;
    }
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct CapturedPlugin;

impl Plugin for CapturedPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<CapturedPieces>()
            .init_resource::<PromotionPending>()
            .add_systems(OnEnter(AppState::Playing), reset_captured)
            .add_systems(OnExit(AppState::Playing),  despawn_panels)
            .add_systems(Update, (
                update_captured_3d,
                show_promotion_overlay,
                handle_promotion_choice,
            ).run_if(in_state(AppState::Playing)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pieces::{PieceColor, PieceType};

    fn w_piece(pt: PieceType) -> Piece {
        Piece { color: PieceColor::White, piece_type: pt, x: 0, y: 0 }
    }

    #[test]
    fn add_and_remove_captured() {
        let mut c = CapturedPieces::default();
        c.add(&w_piece(PieceType::Queen));
        c.add(&w_piece(PieceType::Rook));
        assert_eq!(c.white_captured.len(), 2);
        assert!(c.remove_first(PieceColor::White, PieceType::Queen));
        assert_eq!(c.white_captured.len(), 1);
        assert!(!c.remove_first(PieceColor::White, PieceType::Bishop)); // not present
    }

    #[test]
    fn available_for_promotion_returns_own_captured() {
        let mut c = CapturedPieces::default();
        // White piece captured by Black → stored in white_captured
        c.add(&Piece { color: PieceColor::White, piece_type: PieceType::Rook, x: 0, y: 0 });
        // White promoting player should get back their OWN captured pieces
        assert_eq!(c.available_for_promotion(PieceColor::White), &[PieceType::Rook]);
        assert!(c.available_for_promotion(PieceColor::Black).is_empty());
    }

}
