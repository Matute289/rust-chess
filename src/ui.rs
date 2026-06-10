use bevy::prelude::*;
use crate::analysis::AnalysisReport;
use crate::board::{GameStatus, GameStatusEvent, PlayerTurn};
use crate::pieces::PieceColor;
use crate::state::{AppState, GameConfig, GameMode};

// ─── Components ──────────────────────────────────────────────────────────────

#[derive(Component)] struct StatusBar;
#[derive(Component)] struct GameOverOverlay;
#[derive(Component)] struct BtnRetry;
#[derive(Component)] struct BtnHome;
#[derive(Component)] struct CheckBanner;
#[derive(Component)] struct TurnText;
#[derive(Component)] struct ThinkingBanner;

// ─── In-game HUD ─────────────────────────────────────────────────────────────

fn spawn_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    config: Res<GameConfig>,
) {
    let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");

    let mode_label = match config.mode {
        GameMode::PvP => "Modo: Jugador VS Jugador",
        GameMode::PvC => "Modo: Jugador VS Computadora",
        GameMode::PvL => "Modo: Jugador VS Learning",
    };

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(10.0),
                    top: Val::Px(10.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                ..default()
            },
            StatusBar,
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "Proximo en mover: Blancas",
                    TextStyle { font: font.clone(), font_size: 36.0, color: Color::rgb(0.9, 0.9, 0.9) },
                ),
                TurnText,
            ));
            parent.spawn(TextBundle::from_section(
                mode_label,
                TextStyle { font: font.clone(), font_size: 22.0, color: Color::rgb(0.6, 0.6, 0.8) },
            ));
        });

}

fn despawn_hud(mut commands: Commands, q: Query<Entity, With<StatusBar>>) {
    for e in &q { commands.entity(e).despawn_recursive(); }
}

// ─── Systems ─────────────────────────────────────────────────────────────────

fn update_turn_text(
    turn: Res<PlayerTurn>,
    config: Res<GameConfig>,
    mut query: Query<&mut Text, With<TurnText>>,
) {
    if !turn.is_changed() { return; }
    for mut text in query.iter_mut() {
        let side = match turn.0 {
            PieceColor::White => {
                let is_ai = config.mode != GameMode::PvP && config.player_side == PieceColor::Black;
                if is_ai { "Blancas (Computadora)".to_string() } else { "Blancas".to_string() }
            }
            PieceColor::Black => {
                let is_ai = config.mode != GameMode::PvP && config.player_side == PieceColor::White;
                if is_ai { "Negras (Computadora)".to_string() } else { "Negras".to_string() }
            }
        };
        text.sections[0].value = format!("Proximo en mover: {}", side);
    }
}

fn handle_new_game_btn(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnHome>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &q {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Home);
        }
    }
}

fn handle_status_events(
    mut events:      EventReader<GameStatusEvent>,
    mut commands:    Commands,
    asset_server:    Res<AssetServer>,
    analysis_report: Res<AnalysisReport>,
    overlay_q:       Query<Entity, With<GameOverOverlay>>,
    banner_q:        Query<Entity, With<CheckBanner>>,
) {
    for ev in events.read() {
        // Remove any previous check banner
        for e in &banner_q { commands.entity(e).despawn_recursive(); }

        match &ev.0 {
            GameStatus::Ok => {
                for e in &banner_q { commands.entity(e).despawn_recursive(); }
            }
            GameStatus::Check => {
                let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");
                commands.spawn((
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(100.0),
                            top: Val::Px(90.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        ..default()
                    },
                    CheckBanner,
                ))
                .with_children(|p| {
                    p.spawn(TextBundle::from_section(
                        "  ¡JAQUE!  ",
                        TextStyle { font, font_size: 48.0, color: Color::rgb(1.0, 0.8, 0.1) },
                    ));
                });
            }
            GameStatus::Checkmate { winner } => {
                for e in &overlay_q { commands.entity(e).despawn_recursive(); }
                let winner_str = match winner {
                    PieceColor::White => "¡Jaque Mate! ¡Blancas ganan!",
                    PieceColor::Black => "¡Jaque Mate! ¡Negras ganan!",
                };
                spawn_game_over_overlay(&mut commands, &asset_server, winner_str, false, analysis_report.0.as_ref());
            }
            GameStatus::Stalemate => {
                for e in &overlay_q { commands.entity(e).despawn_recursive(); }
                spawn_game_over_overlay(&mut commands, &asset_server, "¡Empate por ahogado!", true, analysis_report.0.as_ref());
            }
        }
    }
}

fn spawn_game_over_overlay(
    commands:     &mut Commands,
    asset_server: &AssetServer,
    title:        &str,
    is_draw:      bool,
    report:       Option<&chess_engine::GameReport>,
) {
    let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(16.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.85)),
                ..default()
            },
            GameOverOverlay,
        ))
        .with_children(|root| {
            // Title
            root.spawn(TextBundle::from_section(
                title,
                TextStyle { font: font.clone(), font_size: 52.0, color: Color::rgb(1.0, 0.9, 0.2) },
            ));

            // Analysis summary block (if available)
            if let Some(r) = report {
                let s = &r.summary;

                root.spawn(TextBundle::from_section(
                    format!(
                        "Precisión  —  Blancas: {:.0}%   Negras: {:.0}%",
                        s.accuracy_white, s.accuracy_black
                    ),
                    TextStyle { font: font.clone(), font_size: 26.0, color: Color::rgb(0.8, 0.9, 1.0) },
                ));

                root.spawn(TextBundle::from_section(
                    format!(
                        "Blancas: ??{}  ?{}  ⚠{}      Negras: ??{}  ?{}  ⚠{}",
                        s.blunders[0], s.mistakes[0], s.inaccuracies[0],
                        s.blunders[1], s.mistakes[1], s.inaccuracies[1],
                    ),
                    TextStyle { font: font.clone(), font_size: 22.0, color: Color::rgb(0.7, 0.7, 0.7) },
                ));

                let critical: Vec<String> = r.summary.critical_move_indices
                    .iter()
                    .take(3)
                    .filter_map(|&i| r.move_analyses.get(i).map(|a| (i, a)))
                    .map(|(i, a)| {
                        let side = if i % 2 == 0 { "B" } else { "N" };
                        let move_num = i / 2 + 1;
                        format!("Mov {}: {} {}", move_num, side, a.played_move)
                    })
                    .collect();

                if !critical.is_empty() {
                    root.spawn(TextBundle::from_section(
                        format!("Momentos clave: {}", critical.join("  |  ")),
                        TextStyle { font: font.clone(), font_size: 20.0, color: Color::rgb(1.0, 0.5, 0.2) },
                    ));
                }
            }

            // Retry button (only for non-draw)
            if !is_draw {
                root.spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(260.0), height: Val::Px(60.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::rgba(0.1, 0.4, 0.1, 0.9)),
                        ..default()
                    },
                    BtnRetry,
                ))
                .with_children(|p| {
                    p.spawn(TextBundle::from_section(
                        "Reintentar",
                        TextStyle { font: font.clone(), font_size: 30.0, color: Color::rgb(0.9, 0.9, 0.9) },
                    ));
                });
            }

            // Home button
            root.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(260.0), height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.3, 0.1, 0.1, 0.9)),
                    ..default()
                },
                BtnHome,
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section(
                    "Menú principal",
                    TextStyle { font, font_size: 30.0, color: Color::rgb(0.9, 0.9, 0.9) },
                ));
            });
        });
}

fn handle_game_over_buttons(
    retry_q: Query<&Interaction, (Changed<Interaction>, With<BtnRetry>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &retry_q {
        if *interaction == Interaction::Pressed {
            // Re-enter Playing (same config)
            next_state.set(AppState::Playing);
        }
    }
}

fn update_thinking_banner(
    phase: Res<crate::ai::AiPhase>,
    config: Res<GameConfig>,
    banner_q: Query<Entity, With<ThinkingBanner>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !phase.is_changed() { return; }
    if config.mode == GameMode::PvP { return; }

    let is_thinking = phase.is_thinking();
    let has_banner  = !banner_q.is_empty();

    if is_thinking && !has_banner {
        let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");
        commands.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Px(16.0),
                    top: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.05, 0.10, 0.30, 0.88)),
                ..default()
            },
            ThinkingBanner,
        ))
        .with_children(|p| {
            p.spawn(TextBundle::from_section(
                "Pensando...",
                TextStyle { font, font_size: 28.0, color: Color::rgb(0.55, 0.75, 1.0) },
            ));
        });
    } else if !is_thinking && has_banner {
        for e in &banner_q { commands.entity(e).despawn_recursive(); }
    }
}

fn despawn_thinking_banner(
    mut commands: Commands,
    q: Query<Entity, With<ThinkingBanner>>,
) {
    for e in &q { commands.entity(e).despawn_recursive(); }
}

fn despawn_game_over_overlay(
    mut commands: Commands,
    q1: Query<Entity, With<GameOverOverlay>>,
    q2: Query<Entity, With<CheckBanner>>,
) {
    for e in q1.iter().chain(q2.iter()) { commands.entity(e).despawn_recursive(); }
}


// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Playing), spawn_hud)
            .add_systems(OnExit(AppState::Playing), (despawn_hud, despawn_game_over_overlay, despawn_thinking_banner))
            .add_systems(Update, (
                update_turn_text,
                update_thinking_banner,
                handle_new_game_btn,
                handle_status_events,
                handle_game_over_buttons,
            ).run_if(in_state(AppState::Playing)));
    }
}
