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
        match promoting_color {
            PieceColor::White => &self.white_captured,
            PieceColor::Black => &self.black_captured,
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

#[derive(Component)] pub struct CapturedPanelRoot;
#[derive(Component)] struct PromotionOverlayRoot;
#[derive(Component)] pub struct PromotionBtn(pub PieceType);

// ─── Piece symbol helpers ─────────────────────────────────────────────────────

fn piece_symbol(pt: PieceType) -> &'static str {
    match pt {
        PieceType::Queen  => "♛",
        PieceType::Rook   => "♜",
        PieceType::Bishop => "♝",
        PieceType::Knight => "♞",
        PieceType::Pawn   => "♟",
        PieceType::King   => "♚",
    }
}

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

fn format_pieces(pieces: &[PieceType]) -> String {
    let mut counts: Vec<(PieceType, usize)> = Vec::new();
    for &pt in pieces {
        if let Some(entry) = counts.iter_mut().find(|(t, _)| *t == pt) {
            entry.1 += 1;
        } else {
            counts.push((pt, 1));
        }
    }
    counts.iter()
        .map(|(pt, n)| if *n > 1 {
            format!("{}{}", piece_symbol(*pt), n)
        } else {
            piece_symbol(*pt).to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
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
    panels: Query<Entity, With<CapturedPanelRoot>>,
    overlay: Query<Entity, With<PromotionOverlayRoot>>,
) {
    for e in panels.iter().chain(overlay.iter()) {
        commands.entity(e).despawn_recursive();
    }
}

fn update_captured_panels(
    mut commands: Commands,
    captured: Res<CapturedPieces>,
    panels: Query<Entity, With<CapturedPanelRoot>>,
    asset_server: Res<AssetServer>,
) {
    if !captured.is_changed() { return; }
    for e in &panels { commands.entity(e).despawn_recursive(); }

    let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");
    let header_style = TextStyle { font: font.clone(), font_size: 18.0, color: Color::rgb(0.6, 0.6, 0.8) };
    let body_style   = TextStyle { font: font.clone(), font_size: 22.0, color: Color::rgb(0.9, 0.9, 0.9) };

    let white_str = format_pieces(&captured.white_captured);
    let black_str = format_pieces(&captured.black_captured);

    // Left panel: pieces captured from White (taken by Black)
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                top: Val::Percent(20.0),
                width: Val::Px(150.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            ..default()
        },
        CapturedPanelRoot,
    )).with_children(|p| {
        p.spawn(TextBundle::from_section("Negras capturaron:", header_style.clone()));
        p.spawn(TextBundle::from_section(
            if white_str.is_empty() { "–".to_string() } else { white_str },
            body_style.clone(),
        ));
    });

    // Right panel: pieces captured from Black (taken by White)
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(8.0),
                top: Val::Percent(20.0),
                width: Val::Px(150.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            ..default()
        },
        CapturedPanelRoot,
    )).with_children(|p| {
        p.spawn(TextBundle::from_section("Blancas capturaron:", TextStyle { font: font.clone(), font_size: 18.0, color: Color::rgb(0.8, 0.6, 0.6) }));
        p.spawn(TextBundle::from_section(
            if black_str.is_empty() { "–".to_string() } else { black_str },
            body_style,
        ));
    });
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
            format!("{} {}", piece_symbol(pt), piece_name(pt)),
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
) {
    for (interaction, btn) in &q {
        if *interaction != Interaction::Pressed { continue; }

        let pawn_entity = match promotion.pawn_entity { Some(e) => e, None => continue };
        let color       = match promotion.color       { Some(c) => c, None => continue };

        // Apply promotion
        if let Ok((_, mut piece)) = pieces_q.get_mut(pawn_entity) {
            piece.piece_type = btn.0;
        }
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
            }
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
                update_captured_panels,
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
    fn available_for_promotion_returns_own_color() {
        let mut c = CapturedPieces::default();
        c.add(&w_piece(PieceType::Rook)); // White piece captured → in white_captured
        // White promoting player gets back White pieces
        assert_eq!(c.available_for_promotion(PieceColor::White), &[PieceType::Rook]);
        assert!(c.available_for_promotion(PieceColor::Black).is_empty());
    }

    #[test]
    fn format_pieces_groups_duplicates() {
        let pieces = vec![PieceType::Pawn, PieceType::Pawn, PieceType::Rook];
        let s = format_pieces(&pieces);
        assert!(s.contains('2')); // "♟2 ♜" or similar
    }
}
