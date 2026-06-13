use bevy::prelude::*;
use crate::analysis::{AnalysisReport, GameNarrative};
use crate::board::{GameStatus, GameStatusEvent, PlayerTurn};
use crate::pieces::PieceColor;
use crate::adaptive_ai::AdaptiveAiProfile;
use crate::state::{AppState, GameConfig, GameMode, PvLMode};
use crate::suggestion::BtnSuggest;

// ─── Components ──────────────────────────────────────────────────────────────

#[derive(Component)] struct StatusBar;
#[derive(Component)] struct GameOverOverlay;
#[derive(Component)] struct BtnRetry;
#[derive(Component)] struct BtnHome;
#[derive(Component)] struct BtnPvLHub;
#[derive(Component)] struct CheckBanner;
#[derive(Component)] struct TurnText;
#[derive(Component)] struct ThinkingBanner;
#[derive(Component)] struct BoardLabel { world_pos: Vec3 }
#[derive(Component)] struct TimerText;

#[derive(Resource, Default)]
struct TurnTimer {
    remaining: f32,
    initial:   f32,
    disabled:  bool,
}

// ─── In-game HUD ─────────────────────────────────────────────────────────────

fn spawn_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    config:   Res<GameConfig>,
    session:  Res<crate::auth::UserSession>,
    adaptive: Res<AdaptiveAiProfile>,
) {
    let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");

    let mode_label = match config.mode {
        GameMode::PvP    => "Modo: Jugador VS Jugador",
        GameMode::PvC    => "Modo: Jugador VS Computadora",
        GameMode::PvL    => "Modo: Jugador VS Learning",
        GameMode::Lesson => "Modo: Lección",
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
            if config.mode == GameMode::PvL && config.pvl_mode == PvLMode::Adaptativa {
                let elo_label = if adaptive.loaded {
                    format!("IA Nivel {} (ELO {})", adaptive.elo_to_depth(), adaptive.elo_estimate)
                } else {
                    "IA Adaptativa (cargando…)".to_string()
                };
                parent.spawn(TextBundle::from_section(
                    elo_label,
                    TextStyle { font: font.clone(), font_size: 18.0, color: Color::rgb(0.8, 0.65, 0.3) },
                ));
            }
            if let Some(name) = &session.display_name {
                parent.spawn(TextBundle::from_section(
                    format!("Jugador: {}", name),
                    TextStyle { font: font.clone(), font_size: 18.0, color: Color::srgb(0.5, 0.8, 0.5) },
                ));
            }
            if config.timer_secs.is_some() {
                parent.spawn((
                    TextBundle::from_section(
                        "--:--",
                        TextStyle { font: font.clone(), font_size: 42.0, color: Color::rgb(0.9, 0.9, 0.9) },
                    ),
                    TimerText,
                ));
            }
        });

    if config.mode == GameMode::PvL {
        commands.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Px(14.0),
                    top: Val::Px(14.0),
                    ..default()
                },
                z_index: ZIndex::Global(15),
                ..default()
            },
            StatusBar,
        ))
        .with_children(|p| {
            p.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(120.0),
                        height: Val::Px(44.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.10, 0.20, 0.55, 0.90)),
                    border_color: BorderColor(Color::rgba(0.35, 0.55, 1.00, 0.60)),
                    ..default()
                },
                BtnSuggest,
            ))
            .with_children(|btn| {
                btn.spawn(TextBundle::from_section(
                    "Sugerir",
                    TextStyle { font, font_size: 22.0, color: Color::rgb(0.85, 0.92, 1.00) },
                ));
            });
        });
    }

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
    home_q: Query<&Interaction, (Changed<Interaction>, With<BtnHome>)>,
    pvl_q:  Query<&Interaction, (Changed<Interaction>, With<BtnPvLHub>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &home_q {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Home);
        }
    }
    for interaction in &pvl_q {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::PvLHub);
        }
    }
}

fn handle_status_events(
    mut events:      EventReader<GameStatusEvent>,
    mut commands:    Commands,
    asset_server:    Res<AssetServer>,
    analysis_report: Res<AnalysisReport>,
    narrative:       Res<GameNarrative>,
    config:          Res<GameConfig>,
    overlay_q:       Query<Entity, With<GameOverOverlay>>,
    banner_q:        Query<Entity, With<CheckBanner>>,
    mut timer:       ResMut<TurnTimer>,
) {
    for ev in events.read() {
        for e in &banner_q { commands.entity(e).despawn_recursive(); }

        match &ev.0 {
            GameStatus::Ok => {}
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
                timer.disabled = true;
                for e in &overlay_q { commands.entity(e).despawn_recursive(); }
                let winner_str = match winner {
                    PieceColor::White => "¡Jaque Mate! ¡Blancas ganan!",
                    PieceColor::Black => "¡Jaque Mate! ¡Negras ganan!",
                };
                spawn_game_over_overlay(&mut commands, &asset_server, winner_str, false,
                    analysis_report.0.as_ref(), narrative.0.as_deref(), config.mode);
            }
            GameStatus::Stalemate => {
                timer.disabled = true;
                for e in &overlay_q { commands.entity(e).despawn_recursive(); }
                spawn_game_over_overlay(&mut commands, &asset_server, "¡Empate por ahogado!", true,
                    analysis_report.0.as_ref(), narrative.0.as_deref(), config.mode);
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
    narrative:    Option<&str>,
    game_mode:    GameMode,
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

                // Stats table: icon | label | white | black
                root.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|table| {
                    // Header row
                    table.spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(8.0),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|row| {
                        // spacer for icon+label columns
                        row.spawn(NodeBundle { style: Style { width: Val::Px(164.0), ..default() }, ..default() });
                        for header in ["Blancas", "Negras"] {
                            row.spawn(NodeBundle {
                                style: Style { width: Val::Px(64.0), justify_content: JustifyContent::Center, ..default() },
                                ..default()
                            })
                            .with_children(|p| {
                                p.spawn(TextBundle::from_section(header, TextStyle {
                                    font: font.clone(), font_size: 17.0, color: Color::rgb(0.65, 0.65, 0.75),
                                }));
                            });
                        }
                    });

                    // Data rows
                    let rows: [(&str, &str, u8, u8); 3] = [
                        ("icons/blunder.png",    "Graves",    s.blunders[0],     s.blunders[1]),
                        ("icons/mistake.png",    "Errores",   s.mistakes[0],     s.mistakes[1]),
                        ("icons/inaccuracy.png", "Inexactos", s.inaccuracies[0], s.inaccuracies[1]),
                    ];
                    for (icon, label, white_n, black_n) in rows {
                        table.spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(8.0),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn(ImageBundle {
                                style: Style { width: Val::Px(28.0), height: Val::Px(28.0), ..default() },
                                image: UiImage::new(asset_server.load(icon)),
                                ..default()
                            });
                            row.spawn(NodeBundle {
                                style: Style { width: Val::Px(128.0), ..default() },
                                ..default()
                            })
                            .with_children(|p| {
                                p.spawn(TextBundle::from_section(label, TextStyle {
                                    font: font.clone(), font_size: 20.0, color: Color::rgb(0.75, 0.75, 0.75),
                                }));
                            });
                            for count in [white_n, black_n] {
                                row.spawn(NodeBundle {
                                    style: Style { width: Val::Px(64.0), justify_content: JustifyContent::Center, ..default() },
                                    ..default()
                                })
                                .with_children(|p| {
                                    p.spawn(TextBundle::from_section(count.to_string(), TextStyle {
                                        font: font.clone(), font_size: 22.0, color: Color::rgb(0.88, 0.88, 0.92),
                                    }));
                                });
                            }
                        });
                    }
                });

                if let Some(narr) = narrative {
                    root.spawn(NodeBundle {
                        style: Style {
                            max_width: Val::Px(600.0),
                            padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|c| {
                        c.spawn(TextBundle::from_section(
                            narr,
                            TextStyle { font: font.clone(), font_size: 18.0, color: Color::rgb(0.90, 0.88, 0.75) },
                        ));
                    });
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

            // Navigation buttons
            if game_mode == GameMode::PvL {
                root.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(16.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(220.0), height: Val::Px(60.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: BackgroundColor(Color::rgba(0.1, 0.20, 0.50, 0.9)),
                            ..default()
                        },
                        BtnPvLHub,
                    ))
                    .with_children(|p| {
                        p.spawn(TextBundle::from_section(
                            "Menu Learning",
                            TextStyle { font: font.clone(), font_size: 26.0, color: Color::rgb(0.9, 0.9, 0.9) },
                        ));
                    });
                    row.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(220.0), height: Val::Px(60.0),
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
                            TextStyle { font, font_size: 26.0, color: Color::rgb(0.9, 0.9, 0.9) },
                        ));
                    });
                });
            } else {
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
            }
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

fn init_turn_timer(config: Res<crate::state::GameConfig>, mut timer: ResMut<TurnTimer>) {
    let secs = config.timer_secs.map(|s| s as f32).unwrap_or(0.0);
    timer.initial   = secs;
    timer.remaining = secs;
    timer.disabled  = false;
}

fn tick_timer(
    time:          Res<Time>,
    config:        Res<crate::state::GameConfig>,
    mut timer:     ResMut<TurnTimer>,
    turn:          Res<PlayerTurn>,
    mut status_ev: EventWriter<GameStatusEvent>,
) {
    if config.timer_secs.is_none() || timer.disabled { return; }
    if turn.is_changed() {
        timer.remaining = timer.initial;
        return;
    }
    if timer.remaining <= 0.0 { return; }
    timer.remaining -= time.delta_seconds();
    if timer.remaining <= 0.0 {
        timer.remaining = 0.0;
        timer.disabled  = true;
        let winner = match turn.0 {
            PieceColor::White => PieceColor::Black,
            PieceColor::Black => PieceColor::White,
        };
        status_ev.send(GameStatusEvent(GameStatus::Checkmate { winner }));
    }
}

fn update_timer_text(
    timer: Res<TurnTimer>,
    mut q:  Query<&mut Text, With<TimerText>>,
) {
    if !timer.is_changed() { return; }
    let secs  = timer.remaining.ceil() as u32;
    let color = if secs <= 10 { Color::rgb(1.0, 0.25, 0.25) }
                else if secs <= 30 { Color::rgb(1.0, 0.70, 0.0) }
                else { Color::rgb(0.9, 0.9, 0.9) };
    for mut text in &mut q {
        text.sections[0].value = format!("{:02}:{:02}", secs / 60, secs % 60);
        text.sections[0].style.color = color;
    }
}

fn spawn_board_labels(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let style = TextStyle {
        font,
        font_size: 12.0,
        color: Color::rgba(0.9, 0.9, 0.9, 0.55),
    };

    // File labels a–h: just outside rank-1 edge (x = -0.6)
    for file in 0u8..8 {
        commands.spawn((
            TextBundle {
                text: Text::from_section(((b'a' + file) as char).to_string(), style.clone()),
                style: Style { position_type: PositionType::Absolute, ..default() },
                ..default()
            },
            BoardLabel { world_pos: Vec3::new(-0.6, 0.0, file as f32) },
        ));
    }

    // Rank labels 1–8: just outside file-a edge (z = -0.6)
    for rank in 0u8..8 {
        commands.spawn((
            TextBundle {
                text: Text::from_section(((b'1' + rank) as char).to_string(), style.clone()),
                style: Style { position_type: PositionType::Absolute, ..default() },
                ..default()
            },
            BoardLabel { world_pos: Vec3::new(rank as f32, 0.0, -0.6) },
        ));
    }
}

fn despawn_board_labels(mut commands: Commands, q: Query<Entity, With<BoardLabel>>) {
    for e in &q { commands.entity(e).despawn_recursive(); }
}

fn position_board_labels(
    camera_q:  Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut labels: Query<(&BoardLabel, &mut Style)>,
) {
    let Ok((camera, cam_gt)) = camera_q.get_single() else { return };
    for (label, mut style) in &mut labels {
        if let Some(vp) = camera.world_to_viewport(cam_gt, label.world_pos) {
            style.left = Val::Px(vp.x - 5.0);
            style.top  = Val::Px(vp.y - 6.0);
        }
    }
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
            .init_resource::<TurnTimer>()
            .add_systems(OnEnter(AppState::Playing), (spawn_hud, spawn_board_labels, init_turn_timer))
            .add_systems(OnExit(AppState::Playing), (despawn_hud, despawn_game_over_overlay, despawn_thinking_banner, despawn_board_labels))
            .add_systems(Update, (
                update_turn_text,
                update_thinking_banner,
                handle_new_game_btn,
                handle_status_events,
                handle_game_over_buttons,
                position_board_labels,
                tick_timer,
                update_timer_text,
            ).run_if(in_state(AppState::Playing)));
    }
}
