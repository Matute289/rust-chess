use bevy::prelude::*;
use std::sync::{Arc, Mutex};
use crate::{
    auth::UserSession,
    home::{HomeEntryScreen, HomeScreen, despawn_user_menu, spawn_user_menu},
    state::AppState,
};

// ─── Resources ────────────────────────────────────────────────────────────────

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
enum PvLHubScreen {
    #[default]
    Hub,
    NeedLogin,
}

#[derive(Clone, Default)]
pub struct RecentGameEntry {
    pub id:             String,
    pub result:         String,
    pub accuracy_white: Option<f32>,
    pub accuracy_black: Option<f32>,
    pub blunders:       i32,
    pub mistakes:       i32,
    pub inaccuracies:   i32,
    pub summary:        Option<String>,
}

#[derive(Clone, Default)]
pub struct FetchedStats {
    pub wins:               i64,
    pub losses:             i64,
    pub draws:              i64,
    pub accuracy_avg:       Option<f32>,
    pub blunders_total:     i64,
    pub mistakes_total:     i64,
    pub inaccuracies_total: i64,
    pub recent_games:       Vec<RecentGameEntry>,
}

#[derive(Resource, Clone)]
pub struct StatsFetchState(Arc<Mutex<Option<FetchedStats>>>);
impl Default for StatsFetchState {
    fn default() -> Self { Self(Arc::new(Mutex::new(None))) }
}

#[derive(Resource, Default)]
struct LoadedStats(Option<FetchedStats>);

// ─── Components ──────────────────────────────────────────────────────────────

#[derive(Component)] struct PvLHubRoot;
#[derive(Component)] struct BtnPvLJugar;
#[derive(Component)] struct BtnPvLBack;
#[derive(Component)] struct BtnPvLOAuth(pub &'static str);
#[derive(Component)] pub struct EloTooltipTrigger;
#[derive(Component)] struct EloTooltipPanel;
#[derive(Component)] struct BtnSummary(String);
#[derive(Component)] struct BtnSummaryClose;
#[derive(Component)] struct SummaryPopupOverlay;
#[derive(Component)] struct ScrollableGamesContent;

#[derive(Resource, Default)]
struct SummaryPopupText(Option<String>);

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_btn(parent: &mut ChildBuilder, font: Handle<Font>, text: &str, marker: impl Bundle) {
    parent.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(480.0),
                height: Val::Px(64.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.15, 0.15, 0.28, 0.92)),
            border_color: BorderColor(Color::rgba(0.4, 0.4, 0.6, 0.5)),
            ..default()
        },
        marker,
    ))
    .with_children(|p| {
        p.spawn(TextBundle::from_section(
            text,
            TextStyle { font, font_size: 26.0, color: Color::rgb(0.92, 0.92, 0.92) },
        ));
    });
}

fn make_disabled_btn(parent: &mut ChildBuilder, font: Handle<Font>, text: &str) {
    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Px(480.0),
            height: Val::Px(64.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect { left: Val::Px(24.0), right: Val::Px(24.0), ..default() },
            margin: UiRect::all(Val::Px(8.0)),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        background_color: BackgroundColor(Color::rgba(0.10, 0.10, 0.18, 0.70)),
        border_color: BorderColor(Color::rgba(0.30, 0.30, 0.40, 0.30)),
        ..default()
    })
    .with_children(|p| {
        p.spawn(TextBundle::from_section(
            text,
            TextStyle { font: font.clone(), font_size: 24.0, color: Color::rgb(0.45, 0.45, 0.55) },
        ));
        p.spawn(TextBundle::from_section(
            "Próximamente",
            TextStyle { font, font_size: 16.0, color: Color::rgb(0.38, 0.38, 0.48) },
        ));
    });
}

fn spacer(parent: &mut ChildBuilder, px: f32) {
    parent.spawn(NodeBundle {
        style: Style { height: Val::Px(px), ..default() },
        ..default()
    });
}

fn build_table_row(
    parent: &mut ChildBuilder,
    font: Handle<Font>,
    cells: &[(&str, Color)],
    col_widths: &[f32],
    font_size: f32,
) {
    parent.spawn(NodeBundle {
        style: Style { flex_direction: FlexDirection::Row, ..default() },
        ..default()
    })
    .with_children(|row| {
        for ((text, color), &width) in cells.iter().zip(col_widths.iter()) {
            row.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(width),
                    padding: UiRect { left: Val::Px(4.0), right: Val::Px(4.0), top: Val::Px(3.0), bottom: Val::Px(3.0) },
                    ..default()
                },
                ..default()
            })
            .with_children(|cell| {
                cell.spawn(TextBundle::from_section(
                    *text,
                    TextStyle { font: font.clone(), font_size, color: *color },
                ));
            });
        }
    });
}

fn build_games_table(parent: &mut ChildBuilder, font: Handle<Font>, games: &[RecentGameEntry]) {
    if games.is_empty() { return; }

    // Separator
    parent.spawn(NodeBundle {
        style: Style {
            height: Val::Px(1.0),
            width: Val::Percent(100.0),
            margin: UiRect { top: Val::Px(6.0), bottom: Val::Px(8.0), ..default() },
            ..default()
        },
        background_color: BackgroundColor(Color::rgba(0.40, 0.40, 0.60, 0.30)),
        ..default()
    });

    let col_widths = [88.0f32, 82.0, 80.0, 44.0, 44.0, 50.0, 52.0];
    let hc = Color::rgb(0.50, 0.50, 0.68);
    let header_cells: &[(&str, Color)] = &[
        ("Partida", hc), ("Resultado", hc), ("Precisión", hc),
        ("G", hc), ("E", hc), ("I", hc), ("", hc),
    ];
    build_table_row(parent, font.clone(), header_cells, &col_widths, 14.0);

    // Clipping viewport — fixed height so the card never grows with more games
    parent.spawn(NodeBundle {
        style: Style {
            height: Val::Px(140.0),
            overflow: Overflow::clip_y(),
            width: Val::Percent(100.0),
            ..default()
        },
        ..default()
    })
    .with_children(|viewport| {
        viewport.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    top: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ..default()
            },
            ScrollableGamesContent,
        ))
        .with_children(|scroller| {
            for game in games {
                let id_short: String = game.id.chars().take(8).collect();

                let (result_text, result_color) = match game.result.as_str() {
                    "win"  => ("Victoria", Color::rgb(0.45, 0.85, 0.50)),
                    "loss" => ("Derrota",  Color::rgb(0.90, 0.42, 0.42)),
                    _      => ("Tablas",   Color::rgb(0.60, 0.60, 0.78)),
                };

                let avg_acc = match (game.accuracy_white, game.accuracy_black) {
                    (Some(w), Some(b)) => format!("{:.1}%", (w + b) / 2.0),
                    (Some(w), None)    => format!("{:.1}%", w),
                    (None,    Some(b)) => format!("{:.1}%", b),
                    _                  => "-".to_string(),
                };

                let blunders_s     = game.blunders.to_string();
                let mistakes_s     = game.mistakes.to_string();
                let inaccuracies_s = game.inaccuracies.to_string();

                let data_cells: &[(&str, Color)] = &[
                    (id_short.as_str(),       Color::rgb(0.65, 0.65, 0.78)),
                    (result_text,             result_color),
                    (avg_acc.as_str(),        Color::rgb(0.80, 0.80, 0.90)),
                    (blunders_s.as_str(),     Color::rgb(0.90, 0.48, 0.48)),
                    (mistakes_s.as_str(),     Color::rgb(0.88, 0.72, 0.38)),
                    (inaccuracies_s.as_str(), Color::rgb(0.70, 0.70, 0.88)),
                ];
                // Data row: first 6 text columns + optional (ver) button
                scroller.spawn(NodeBundle {
                    style: Style { flex_direction: FlexDirection::Row, ..default() },
                    ..default()
                })
                .with_children(|row| {
                    for ((text, color), &width) in data_cells.iter().zip(col_widths.iter()) {
                        row.spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(width),
                                padding: UiRect { left: Val::Px(4.0), right: Val::Px(4.0), top: Val::Px(3.0), bottom: Val::Px(3.0) },
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|cell| {
                            cell.spawn(TextBundle::from_section(
                                *text,
                                TextStyle { font: font.clone(), font_size: 14.0, color: *color },
                            ));
                        });
                    }
                    // (ver) button — last column
                    let ver_width = col_widths[6];
                    if let Some(summary_text) = &game.summary {
                        row.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(ver_width),
                                    height: Val::Px(24.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::rgba(0.15, 0.25, 0.45, 0.85)),
                                border_color: BorderColor(Color::rgba(0.35, 0.45, 0.70, 0.60)),
                                ..default()
                            },
                            BtnSummary(summary_text.clone()),
                        ))
                        .with_children(|p| {
                            p.spawn(TextBundle::from_section(
                                "ver",
                                TextStyle { font: font.clone(), font_size: 13.0, color: Color::rgb(0.70, 0.80, 0.95) },
                            ));
                        });
                    } else {
                        row.spawn(NodeBundle {
                            style: Style { width: Val::Px(ver_width), ..default() },
                            ..default()
                        });
                    }
                });
            }
        });
    });
}

// ─── UI builder ──────────────────────────────────────────────────────────────

fn build_pvl_hub_root(
    commands: &mut Commands,
    asset_server: &AssetServer,
    screen: PvLHubScreen,
    session: &UserSession,
    stats: &Option<FetchedStats>,
) {
    let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.04, 0.04, 0.10, 0.97)),
                ..default()
            },
            PvLHubRoot,
        ))
        .with_children(|root| match screen {
            PvLHubScreen::Hub => {
                root.spawn(TextBundle::from_section(
                    "MODO APRENDIZAJE",
                    TextStyle { font: font.clone(), font_size: 52.0, color: Color::rgb(0.95, 0.92, 0.80) },
                ));
                spacer(root, 28.0);

                // Stats card
                root.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(24.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        min_width: Val::Px(500.0),
                        row_gap: Val::Px(6.0),
                        margin: UiRect { bottom: Val::Px(8.0), ..default() },
                        ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.08, 0.08, 0.18, 0.95)),
                    border_color: BorderColor(Color::rgba(0.40, 0.40, 0.70, 0.50)),
                    ..default()
                })
                .with_children(|card| {
                    let elo = session.elo.unwrap_or(800);

                    // ELO row with tooltip trigger
                    card.spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|elo_row| {
                        elo_row.spawn(TextBundle::from_section(
                            format!("ELO: {}", elo),
                            TextStyle { font: font.clone(), font_size: 34.0, color: Color::rgb(0.95, 0.92, 0.80) },
                        ));
                        // Hoverable (i) tooltip trigger
                        elo_row.spawn((
                            ButtonBundle {
                                style: Style {
                                    padding: UiRect::all(Val::Px(4.0)),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::NONE),
                                border_color: BorderColor(Color::NONE),
                                ..default()
                            },
                            EloTooltipTrigger,
                        ))
                        .with_children(|p| {
                            p.spawn(TextBundle::from_section(
                                "(i)",
                                TextStyle { font: font.clone(), font_size: 17.0, color: Color::rgb(0.45, 0.45, 0.70) },
                            ));
                        });
                    });

                    // Tooltip text — always present, transparent when not hovered
                    card.spawn((
                        TextBundle::from_section(
                            "Sistema de puntuación que mide el nivel relativo de cada jugador\n0-999: principiante  ·  1000-1499: intermedio  ·  1500+: avanzado",
                            TextStyle { font: font.clone(), font_size: 14.0, color: Color::NONE },
                        ),
                        EloTooltipPanel,
                    ));

                    if let Some(s) = stats {
                        let total = s.wins + s.losses + s.draws;
                        card.spawn(TextBundle::from_section(
                            format!("Partidas: {}   V: {}   D: {}   T: {}", total, s.wins, s.losses, s.draws),
                            TextStyle { font: font.clone(), font_size: 22.0, color: Color::rgb(0.70, 0.85, 0.70) },
                        ));
                        let acc = s.accuracy_avg
                            .map(|a| format!("{:.1}%", a))
                            .unwrap_or_else(|| "-".to_string());
                        card.spawn(TextBundle::from_section(
                            format!("Precisión promedio: {}", acc),
                            TextStyle { font: font.clone(), font_size: 22.0, color: Color::rgb(0.80, 0.80, 0.90) },
                        ));
                        card.spawn(TextBundle::from_section(
                            format!(
                                "Graves: {}   Errores: {}   Inexactos: {}",
                                s.blunders_total, s.mistakes_total, s.inaccuracies_total
                            ),
                            TextStyle { font: font.clone(), font_size: 20.0, color: Color::rgb(0.80, 0.65, 0.65) },
                        ));
                        build_games_table(card, font.clone(), &s.recent_games);
                    } else {
                        card.spawn(TextBundle::from_section(
                            "Cargando estadísticas...",
                            TextStyle { font: font.clone(), font_size: 20.0, color: Color::rgb(0.55, 0.55, 0.65) },
                        ));
                    }
                });

                spacer(root, 8.0);
                make_btn(root, font.clone(), "Jugar partida Learning", BtnPvLJugar);
                make_disabled_btn(root, font.clone(), "vs IA Adaptativa");
                make_disabled_btn(root, font.clone(), "Con Sugerencias");
                make_disabled_btn(root, font.clone(), "Currículo de Lecciones");
                spacer(root, 16.0);
                make_btn(root, font.clone(), "← Volver", BtnPvLBack);
            }

            PvLHubScreen::NeedLogin => {
                root.spawn(TextBundle::from_section(
                    "MODO APRENDIZAJE",
                    TextStyle { font: font.clone(), font_size: 52.0, color: Color::rgb(0.95, 0.92, 0.80) },
                ));
                spacer(root, 16.0);
                root.spawn(TextBundle::from_section(
                    "Iniciá sesión para acceder",
                    TextStyle { font: font.clone(), font_size: 26.0, color: Color::rgb(0.65, 0.65, 0.80) },
                ));
                spacer(root, 28.0);
                make_btn(root, font.clone(), "Continuar con Google",  BtnPvLOAuth("Google"));
                make_btn(root, font.clone(), "Continuar con GitHub",  BtnPvLOAuth("GitHub"));
                make_btn(root, font.clone(), "Continuar con Discord", BtnPvLOAuth("Discord"));
                spacer(root, 16.0);
                make_btn(root, font.clone(), "← Volver", BtnPvLBack);
            }
        });
}

// ─── Lifecycle systems ────────────────────────────────────────────────────────

fn setup_pvl_hub(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    session: Res<UserSession>,
    mut screen: ResMut<PvLHubScreen>,
    mut loaded_stats: ResMut<LoadedStats>,
    fetch_state: Res<StatsFetchState>,
) {
    loaded_stats.0 = None;
    if let Ok(mut g) = fetch_state.0.try_lock() { *g = None; }

    if session.is_logged_in() {
        *screen = PvLHubScreen::Hub;
        #[cfg(target_arch = "wasm32")]
        if let Some(jwt) = session.jwt.clone() {
            let arc = fetch_state.0.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(stats) = fetch_stats_async(jwt).await {
                    *arc.lock().unwrap() = Some(stats);
                }
            });
        }
    } else {
        *screen = PvLHubScreen::NeedLogin;
    }

    build_pvl_hub_root(&mut commands, &asset_server, *screen, &session, &loaded_stats.0);
}

fn despawn_pvl_hub(
    mut commands:  Commands,
    q:             Query<Entity, With<PvLHubRoot>>,
    overlay_q:     Query<Entity, With<SummaryPopupOverlay>>,
    mut popup:     ResMut<SummaryPopupText>,
) {
    for e in &q         { commands.entity(e).despawn_recursive(); }
    for e in &overlay_q { commands.entity(e).despawn_recursive(); }
    popup.0 = None;
}

fn poll_stats_result(
    fetch_state: Res<StatsFetchState>,
    mut loaded_stats: ResMut<LoadedStats>,
    screen: Res<PvLHubScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<PvLHubRoot>>,
    asset_server: Res<AssetServer>,
    session: Res<UserSession>,
) {
    if let Ok(mut guard) = fetch_state.0.try_lock() {
        if let Some(stats) = guard.take() {
            loaded_stats.0 = Some(stats);
            if *screen == PvLHubScreen::Hub {
                for e in &root_q { commands.entity(e).despawn_recursive(); }
                build_pvl_hub_root(&mut commands, &asset_server, PvLHubScreen::Hub, &session, &loaded_stats.0);
            }
        }
    }
}

// ─── Button handlers ─────────────────────────────────────────────────────────

fn highlight_pvl_buttons(
    mut q: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>, Without<EloTooltipTrigger>, Without<BtnSummaryClose>, Without<BtnSummary>),
    >,
) {
    for (i, mut color) in &mut q {
        *color = match i {
            Interaction::Pressed => BackgroundColor(Color::rgba(0.28, 0.28, 0.55, 0.97)),
            Interaction::Hovered => BackgroundColor(Color::rgba(0.22, 0.22, 0.45, 0.95)),
            Interaction::None    => BackgroundColor(Color::rgba(0.15, 0.15, 0.28, 0.92)),
        };
    }
}

fn handle_elo_tooltip(
    q: Query<&Interaction, (Changed<Interaction>, With<EloTooltipTrigger>)>,
    mut tooltip_q: Query<&mut Text, With<EloTooltipPanel>>,
) {
    for i in &q {
        if let Ok(mut text) = tooltip_q.get_single_mut() {
            text.sections[0].style.color = match i {
                Interaction::Hovered | Interaction::Pressed => Color::rgba(0.82, 0.82, 0.95, 0.95),
                Interaction::None => Color::NONE,
            };
        }
    }
}

fn handle_pvl_jugar(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnPvLJugar>)>,
    mut entry: ResMut<HomeEntryScreen>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            entry.0 = HomeScreen::ColorSelect;
            next_state.set(AppState::Home);
        }
    }
}

fn handle_pvl_oauth(
    q: Query<(&Interaction, &BtnPvLOAuth), Changed<Interaction>>,
) {
    for (i, btn) in &q {
        if *i == Interaction::Pressed {
            #[cfg(target_arch = "wasm32")]
            {
                let provider = btn.0.to_lowercase();
                let url = format!(
                    "https://rustchess.greenmountain.dev/api/auth/{}/login",
                    provider
                );
                if let Some(win) = web_sys::window() {
                    let _ = win.location().set_href(&url);
                }
            }
            let _ = btn;
        }
    }
}

fn handle_pvl_back(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnPvLBack>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            next_state.set(AppState::Home);
        }
    }
}

// ─── Summary popup ────────────────────────────────────────────────────────────

fn spawn_summary_popup(commands: &mut Commands, asset_server: &AssetServer, text: &str) {
    let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width:         Val::Percent(100.0),
                height:        Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items:     AlignItems::Center,
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.80)),
            z_index: ZIndex::Global(50),
            ..default()
        },
        SummaryPopupOverlay,
    ))
    .with_children(|root| {
        root.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                padding:        UiRect::all(Val::Px(32.0)),
                max_width:      Val::Px(580.0),
                row_gap:        Val::Px(18.0),
                border:         UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.06, 0.06, 0.15, 0.97)),
            border_color: BorderColor(Color::rgba(0.40, 0.40, 0.70, 0.60)),
            ..default()
        })
        .with_children(|panel| {
            panel.spawn(TextBundle::from_section(
                "Análisis de la partida",
                TextStyle { font: font.clone(), font_size: 24.0, color: Color::rgb(0.90, 0.85, 0.70) },
            ));
            panel.spawn(NodeBundle {
                style: Style { max_width: Val::Px(516.0), ..default() },
                ..default()
            })
            .with_children(|c| {
                c.spawn(TextBundle::from_section(
                    text,
                    TextStyle { font: font.clone(), font_size: 17.0, color: Color::rgb(0.85, 0.85, 0.92) },
                ));
            });
            panel.spawn((
                ButtonBundle {
                    style: Style {
                        padding: UiRect { left: Val::Px(36.0), right: Val::Px(36.0), top: Val::Px(12.0), bottom: Val::Px(12.0) },
                        justify_content: JustifyContent::Center,
                        align_items:     AlignItems::Center,
                        align_self:      AlignSelf::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.22, 0.10, 0.10, 0.92)),
                    border_color:     BorderColor(Color::rgba(0.50, 0.20, 0.20, 0.60)),
                    ..default()
                },
                BtnSummaryClose,
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section(
                    "Cerrar",
                    TextStyle { font: font.clone(), font_size: 20.0, color: Color::rgb(0.92, 0.88, 0.88) },
                ));
            });
        });
    });
}

fn handle_summary_btn(
    q:         Query<(&Interaction, &BtnSummary), Changed<Interaction>>,
    mut popup: ResMut<SummaryPopupText>,
) {
    for (i, btn) in &q {
        if *i == Interaction::Pressed {
            popup.0 = Some(btn.0.clone());
        }
    }
}

fn sync_summary_popup(
    popup:       Res<SummaryPopupText>,
    overlay_q:   Query<Entity, With<SummaryPopupOverlay>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !popup.is_changed() { return; }
    for e in &overlay_q { commands.entity(e).despawn_recursive(); }
    if let Some(text) = &popup.0 {
        spawn_summary_popup(&mut commands, &asset_server, text);
    }
}

fn handle_summary_close(
    q:         Query<&Interaction, (Changed<Interaction>, With<BtnSummaryClose>)>,
    mut popup: ResMut<SummaryPopupText>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            popup.0 = None;
        }
    }
}

fn highlight_close_btn(
    mut q: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<BtnSummaryClose>)>,
) {
    for (i, mut color) in &mut q {
        *color = match i {
            Interaction::Pressed => BackgroundColor(Color::rgba(0.50, 0.15, 0.15, 0.97)),
            Interaction::Hovered => BackgroundColor(Color::rgba(0.38, 0.12, 0.12, 0.95)),
            Interaction::None    => BackgroundColor(Color::rgba(0.22, 0.10, 0.10, 0.92)),
        };
    }
}

fn highlight_summary_btn(
    mut q: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<BtnSummary>)>,
) {
    for (i, mut color) in &mut q {
        *color = match i {
            Interaction::Pressed => BackgroundColor(Color::rgba(0.28, 0.48, 0.78, 0.97)),
            Interaction::Hovered => BackgroundColor(Color::rgba(0.22, 0.38, 0.65, 0.95)),
            Interaction::None    => BackgroundColor(Color::rgba(0.15, 0.25, 0.45, 0.85)),
        };
    }
}

// ─── Stats fetch (WASM only) ──────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
async fn fetch_stats_async(jwt: String) -> Option<FetchedStats> {
    #[derive(serde::Deserialize)]
    struct GameRow {
        id:             String,
        result:         String,
        accuracy_white: Option<f32>,
        accuracy_black: Option<f32>,
        blunders:       i32,
        mistakes:       i32,
        inaccuracies:   i32,
        summary:        Option<String>,
    }

    #[derive(serde::Deserialize)]
    struct Resp {
        wins:               i64,
        losses:             i64,
        draws:              i64,
        accuracy_avg:       Option<f32>,
        blunders_total:     i64,
        mistakes_total:     i64,
        inaccuracies_total: i64,
        recent_games:       Vec<GameRow>,
    }

    let resp = gloo_net::http::Request::get(
        "https://rustchess.greenmountain.dev/api/stats",
    )
    .header("Authorization", &format!("Bearer {}", jwt))
    .send()
    .await
    .ok()?;

    let r: Resp = resp.json().await.ok()?;
    Some(FetchedStats {
        wins:               r.wins,
        losses:             r.losses,
        draws:              r.draws,
        accuracy_avg:       r.accuracy_avg,
        blunders_total:     r.blunders_total,
        mistakes_total:     r.mistakes_total,
        inaccuracies_total: r.inaccuracies_total,
        recent_games: r.recent_games.into_iter().map(|g| RecentGameEntry {
            id:             g.id,
            result:         g.result,
            accuracy_white: g.accuracy_white,
            accuracy_black: g.accuracy_black,
            blunders:       g.blunders,
            mistakes:       g.mistakes,
            inaccuracies:   g.inaccuracies,
            summary:        g.summary,
        }).collect(),
    })
}

fn scroll_games_table(
    mut wheel_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut q: Query<&mut Style, With<ScrollableGamesContent>>,
) {
    use bevy::input::mouse::MouseScrollUnit;
    let mut delta = 0.0f32;
    for ev in wheel_events.read() {
        delta += match ev.unit {
            MouseScrollUnit::Line  => ev.y * 24.0,
            MouseScrollUnit::Pixel => ev.y,
        };
    }
    if delta == 0.0 { return; }
    for mut style in &mut q {
        let current = match style.top { Val::Px(v) => v, _ => 0.0 };
        style.top = Val::Px((current + delta).min(0.0).max(-300.0));
    }
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct PvLHubPlugin;

impl Plugin for PvLHubPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PvLHubScreen>()
            .init_resource::<StatsFetchState>()
            .init_resource::<LoadedStats>()
            .init_resource::<SummaryPopupText>()
            .add_systems(OnEnter(AppState::PvLHub), (setup_pvl_hub, spawn_user_menu).chain())
            .add_systems(OnExit(AppState::PvLHub),  (despawn_pvl_hub, despawn_user_menu))
            .add_systems(Update, (
                highlight_pvl_buttons,
                highlight_close_btn,
                highlight_summary_btn,
                handle_elo_tooltip,
                poll_stats_result,
                handle_pvl_jugar,
                handle_pvl_oauth,
                handle_pvl_back,
                handle_summary_btn,
                sync_summary_popup,
                handle_summary_close,
                scroll_games_table,
            ).run_if(in_state(AppState::PvLHub)));
    }
}
