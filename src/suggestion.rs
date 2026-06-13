use bevy::prelude::*;
use chess_engine::{DifficultyConfig, Search, SearchResult};
use crate::{
    ai::build_fen_ep,
    board::{CastlingState, EnPassantTarget, GameHistory, PlayerTurn},
    pieces::{Piece, PieceType},
    state::{AppState, GameConfig, Suggestion},
};

// ─── Components ──────────────────────────────────────────────────────────────

#[derive(Component)] pub struct BtnSuggest;
#[derive(Component)] struct BtnDismissSuggestion;
#[derive(Component)] struct SuggestionPanel;

// ─── Systems ─────────────────────────────────────────────────────────────────

fn reset_suggestion(mut s: ResMut<Suggestion>) { s.clear(); }

fn handle_suggest_btn(
    q:          Query<&Interaction, (Changed<Interaction>, With<BtnSuggest>)>,
    mut s:      ResMut<Suggestion>,
    config:     Res<GameConfig>,
    turn:       Res<PlayerTurn>,
    pieces_q:   Query<&Piece>,
    castling:   Res<CastlingState>,
    ep:         Res<EnPassantTarget>,
) {
    for i in &q {
        if *i != Interaction::Pressed { continue; }
        if turn.0 != config.player_side { continue; }

        let pieces: Vec<Piece> = pieces_q.iter().copied().collect();
        let fen = build_fen_ep(&pieces, turn.0, &castling, ep.0);
        let Ok(pos) = chess_engine::Position::from_fen(&fen) else { continue };

        let depth_cfg = DifficultyConfig { max_depth: 5, max_nodes: 1_000_000, random_factor: 0.0 };
        let SearchResult::EngineMove(mv, score) = Search::new().best_move(&pos, &depth_cfg);

        // Use engine square methods directly (same convention as ai.rs):
        //   engine.rank() → piece.x (Bevy rank, 0 = rank 1)
        //   engine.file() → piece.y (Bevy file, 0 = file a)
        let from_rank = mv.from_sq().rank();  // = piece.x
        let from_file = mv.from_sq().file();  // = piece.y
        let to_rank   = mv.to_sq().rank();
        let to_file   = mv.to_sq().file();

        // Skip null move
        if mv.from_sq() == mv.to_sq() { continue; }

        let piece_name = pieces.iter()
            .find(|p| p.x == from_rank && p.y == from_file)
            .map(|p| match p.piece_type {
                PieceType::Pawn   => "peón",
                PieceType::Knight => "caballo",
                PieceType::Bishop => "alfil",
                PieceType::Rook   => "torre",
                PieceType::Queen  => "dama",
                PieceType::King   => "rey",
            })
            .unwrap_or("pieza");

        let from_label = format!("{}{}", (b'a' + from_file) as char, from_rank + 1);
        let to_label   = format!("{}{}", (b'a' + to_file)   as char, to_rank   + 1);

        let quality = if score >= 150 { "Movimiento excelente" }
                      else if score >= 50 { "Buena jugada" }
                      else if score >= 0  { "Jugada sólida" }
                      else { "Mejor opción disponible" };

        s.from_sq = Some((from_rank, from_file));
        s.to_sq   = Some((to_rank,   to_file));
        s.text    = Some(format!("Mueve el {} de {} a {}  —  {}", piece_name, from_label, to_label, quality));
    }
}

fn handle_dismiss_btn(
    q:     Query<&Interaction, (Changed<Interaction>, With<BtnDismissSuggestion>)>,
    mut s: ResMut<Suggestion>,
) {
    for i in &q {
        if *i == Interaction::Pressed { s.clear(); }
    }
}

fn clear_on_move(history: Res<GameHistory>, mut s: ResMut<Suggestion>) {
    if !s.is_active() { return; }
    if history.is_changed() { s.clear(); }
}

fn sync_suggestion_panel(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    s:            Res<Suggestion>,
    panel_q:      Query<Entity, With<SuggestionPanel>>,
) {
    if !s.is_changed() { return; }
    for e in &panel_q { commands.entity(e).despawn_recursive(); }
    let Some(ref text) = s.text else { return };

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(16.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            z_index: ZIndex::Global(20),
            ..default()
        },
        SuggestionPanel,
    ))
    .with_children(|root| {
        root.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(14.0),
                padding: UiRect { left: Val::Px(20.0), right: Val::Px(12.0), top: Val::Px(10.0), bottom: Val::Px(10.0) },
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.04, 0.10, 0.28, 0.93)),
            border_color: BorderColor(Color::rgba(0.30, 0.52, 0.92, 0.65)),
            ..default()
        })
        .with_children(|panel| {
            panel.spawn(TextBundle::from_section(
                text.as_str(),
                TextStyle { font: font.clone(), font_size: 20.0, color: Color::rgb(0.82, 0.90, 1.00) },
            ));
            panel.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(28.0), height: Val::Px(28.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.35, 0.10, 0.10, 0.85)),
                    ..default()
                },
                BtnDismissSuggestion,
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section(
                    "X",
                    TextStyle { font, font_size: 15.0, color: Color::rgb(0.9, 0.6, 0.6) },
                ));
            });
        });
    });
}

fn highlight_suggest_btn(
    config: Res<GameConfig>,
    turn:   Res<PlayerTurn>,
    mut q:  Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<BtnSuggest>)>,
) {
    let is_player_turn = turn.0 == config.player_side;
    for (i, mut color) in &mut q {
        *color = if !is_player_turn {
            BackgroundColor(Color::rgba(0.15, 0.15, 0.28, 0.60))
        } else {
            match i {
                Interaction::Pressed => BackgroundColor(Color::rgba(0.22, 0.38, 0.85, 0.97)),
                Interaction::Hovered => BackgroundColor(Color::rgba(0.16, 0.30, 0.70, 0.95)),
                Interaction::None    => BackgroundColor(Color::rgba(0.10, 0.20, 0.55, 0.90)),
            }
        };
    }
}

fn despawn_suggestion_panel(
    mut commands: Commands,
    panel_q: Query<Entity, With<SuggestionPanel>>,
) {
    for e in &panel_q { commands.entity(e).despawn_recursive(); }
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct SuggestionPlugin;

impl Plugin for SuggestionPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<Suggestion>()
            .add_systems(OnEnter(AppState::Playing), reset_suggestion)
            .add_systems(OnExit(AppState::Playing),  despawn_suggestion_panel)
            .add_systems(Update, (
                handle_suggest_btn,
                handle_dismiss_btn,
                clear_on_move,
                sync_suggestion_panel,
                highlight_suggest_btn,
            ).run_if(in_state(AppState::Playing)));
    }
}
