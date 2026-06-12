use bevy::prelude::*;
use std::sync::{Arc, Mutex};
use crate::{
    ai::Difficulty,
    auth::UserSession,
    pieces::PieceColor,
    state::{AppState, GameConfig},
};

// ─── Constants ────────────────────────────────────────────────────────────────

const TIMER_PRESETS_MINS: &[u32] = &[
    1, 3, 5, 10, 15, 20, 25, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120,
];

// ─── Resources ────────────────────────────────────────────────────────────────

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum PvLHubScreen {
    #[default]
    Hub,
    NeedLogin,
    ColorSelect,
    DifficultySelect,
    TimerSelect,
}

#[derive(Resource)]
struct PvLTimerIdx(usize);
impl Default for PvLTimerIdx {
    fn default() -> Self { Self(2) } // 5 min
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
#[derive(Component)] struct BtnPvLSideWhite;
#[derive(Component)] struct BtnPvLSideBlack;
#[derive(Component)] struct BtnPvLDifficulty(Difficulty);
#[derive(Component)] struct BtnPvLTimerDown;
#[derive(Component)] struct BtnPvLTimerUp;
#[derive(Component)] struct BtnPvLNoTimer;
#[derive(Component)] struct BtnPvLPlay;

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

// ─── UI builder ──────────────────────────────────────────────────────────────

fn build_pvl_hub_root(
    commands: &mut Commands,
    asset_server: &AssetServer,
    screen: PvLHubScreen,
    timer_idx: usize,
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
                        min_width: Val::Px(480.0),
                        row_gap: Val::Px(8.0),
                        margin: UiRect { bottom: Val::Px(8.0), ..default() },
                        ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.08, 0.08, 0.18, 0.95)),
                    border_color: BorderColor(Color::rgba(0.40, 0.40, 0.70, 0.50)),
                    ..default()
                })
                .with_children(|card| {
                    let elo = session.elo.unwrap_or(800);
                    card.spawn(TextBundle::from_section(
                        format!("ELO: {}", elo),
                        TextStyle { font: font.clone(), font_size: 34.0, color: Color::rgb(0.95, 0.92, 0.80) },
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

            PvLHubScreen::ColorSelect => {
                root.spawn(TextBundle::from_section(
                    "¿Con qué color jugás?",
                    TextStyle { font: font.clone(), font_size: 36.0, color: Color::rgb(0.7, 0.7, 0.85) },
                ));
                spacer(root, 24.0);
                make_btn(root, font.clone(), "Blancas", BtnPvLSideWhite);
                make_btn(root, font.clone(), "Negras",  BtnPvLSideBlack);
                spacer(root, 12.0);
                make_btn(root, font.clone(), "← Volver", BtnPvLBack);
            }

            PvLHubScreen::DifficultySelect => {
                root.spawn(TextBundle::from_section(
                    "Elegí la dificultad",
                    TextStyle { font: font.clone(), font_size: 36.0, color: Color::rgb(0.7, 0.7, 0.85) },
                ));
                spacer(root, 20.0);
                for d in [
                    Difficulty::Principiante,
                    Difficulty::Facil,
                    Difficulty::Medio,
                    Difficulty::Dificil,
                    Difficulty::Pro,
                ] {
                    make_btn(root, font.clone(), d.label(), BtnPvLDifficulty(d));
                }
                spacer(root, 12.0);
                make_btn(root, font.clone(), "← Volver", BtnPvLBack);
            }

            PvLHubScreen::TimerSelect => {
                root.spawn(TextBundle::from_section(
                    "¿Cuánto tiempo por turno?",
                    TextStyle { font: font.clone(), font_size: 32.0, color: Color::rgb(0.7, 0.7, 0.85) },
                ));
                spacer(root, 28.0);

                root.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(72.0), height: Val::Px(72.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::rgba(0.15, 0.15, 0.28, 0.92)),
                            border_color: BorderColor(Color::rgba(0.4, 0.4, 0.6, 0.5)),
                            ..default()
                        },
                        BtnPvLTimerDown,
                    ))
                    .with_children(|p| {
                        p.spawn(TextBundle::from_section("<", TextStyle {
                            font: font.clone(), font_size: 32.0, color: Color::rgb(0.92, 0.92, 0.92),
                        }));
                    });

                    let mins = TIMER_PRESETS_MINS[timer_idx];
                    let label = if mins == 1 { "1 min".to_string() } else { format!("{} min", mins) };
                    row.spawn(NodeBundle {
                        style: Style {
                            width: Val::Px(200.0), height: Val::Px(72.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|p| {
                        p.spawn(TextBundle::from_section(label, TextStyle {
                            font: font.clone(), font_size: 40.0, color: Color::rgb(0.95, 0.92, 0.80),
                        }));
                    });

                    row.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(72.0), height: Val::Px(72.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::rgba(0.15, 0.15, 0.28, 0.92)),
                            border_color: BorderColor(Color::rgba(0.4, 0.4, 0.6, 0.5)),
                            ..default()
                        },
                        BtnPvLTimerUp,
                    ))
                    .with_children(|p| {
                        p.spawn(TextBundle::from_section(">", TextStyle {
                            font: font.clone(), font_size: 32.0, color: Color::rgb(0.92, 0.92, 0.92),
                        }));
                    });
                });

                spacer(root, 20.0);
                make_btn(root, font.clone(), "Sin reloj", BtnPvLNoTimer);
                make_btn(root, font.clone(), "Jugar",     BtnPvLPlay);
                spacer(root, 8.0);
                make_btn(root, font.clone(), "← Volver",  BtnPvLBack);
            }
        });
}

fn rebuild_pvl_hub(
    commands: &mut Commands,
    asset_server: &AssetServer,
    root_q: &Query<Entity, With<PvLHubRoot>>,
    screen: PvLHubScreen,
    timer_idx: usize,
    session: &UserSession,
    stats: &Option<FetchedStats>,
) {
    for e in root_q { commands.entity(e).despawn_recursive(); }
    build_pvl_hub_root(commands, asset_server, screen, timer_idx, session, stats);
}

// ─── Lifecycle systems ────────────────────────────────────────────────────────

fn setup_pvl_hub(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    session: Res<UserSession>,
    mut screen: ResMut<PvLHubScreen>,
    mut loaded_stats: ResMut<LoadedStats>,
    timer_idx: Res<PvLTimerIdx>,
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

    build_pvl_hub_root(&mut commands, &asset_server, *screen, timer_idx.0, &session, &loaded_stats.0);
}

fn despawn_pvl_hub(mut commands: Commands, q: Query<Entity, With<PvLHubRoot>>) {
    for e in &q { commands.entity(e).despawn_recursive(); }
}

fn poll_stats_result(
    fetch_state: Res<StatsFetchState>,
    mut loaded_stats: ResMut<LoadedStats>,
    screen: Res<PvLHubScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<PvLHubRoot>>,
    asset_server: Res<AssetServer>,
    session: Res<UserSession>,
    timer_idx: Res<PvLTimerIdx>,
) {
    if let Ok(mut guard) = fetch_state.0.try_lock() {
        if let Some(stats) = guard.take() {
            loaded_stats.0 = Some(stats);
            if *screen == PvLHubScreen::Hub {
                for e in &root_q { commands.entity(e).despawn_recursive(); }
                build_pvl_hub_root(&mut commands, &asset_server, PvLHubScreen::Hub, timer_idx.0, &session, &loaded_stats.0);
            }
        }
    }
}

// ─── Button handlers ─────────────────────────────────────────────────────────

fn highlight_pvl_buttons(
    mut q: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>,
) {
    for (i, mut color) in &mut q {
        *color = match i {
            Interaction::Pressed => BackgroundColor(Color::rgba(0.28, 0.28, 0.55, 0.97)),
            Interaction::Hovered => BackgroundColor(Color::rgba(0.22, 0.22, 0.45, 0.95)),
            Interaction::None    => BackgroundColor(Color::rgba(0.15, 0.15, 0.28, 0.92)),
        };
    }
}

fn handle_pvl_jugar(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnPvLJugar>)>,
    mut screen: ResMut<PvLHubScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<PvLHubRoot>>,
    asset_server: Res<AssetServer>,
    timer_idx: Res<PvLTimerIdx>,
    session: Res<UserSession>,
    loaded_stats: Res<LoadedStats>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            *screen = PvLHubScreen::ColorSelect;
            rebuild_pvl_hub(&mut commands, &asset_server, &root_q, *screen, timer_idx.0, &session, &loaded_stats.0);
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

fn handle_pvl_color_select(
    white_q: Query<&Interaction, (Changed<Interaction>, With<BtnPvLSideWhite>)>,
    black_q: Query<&Interaction, (Changed<Interaction>, With<BtnPvLSideBlack>)>,
    mut config: ResMut<GameConfig>,
    mut screen: ResMut<PvLHubScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<PvLHubRoot>>,
    asset_server: Res<AssetServer>,
    timer_idx: Res<PvLTimerIdx>,
    session: Res<UserSession>,
    loaded_stats: Res<LoadedStats>,
) {
    for i in &white_q {
        if *i == Interaction::Pressed {
            config.player_side = PieceColor::White;
            *screen = PvLHubScreen::DifficultySelect;
            rebuild_pvl_hub(&mut commands, &asset_server, &root_q, *screen, timer_idx.0, &session, &loaded_stats.0);
        }
    }
    for i in &black_q {
        if *i == Interaction::Pressed {
            config.player_side = PieceColor::Black;
            *screen = PvLHubScreen::DifficultySelect;
            rebuild_pvl_hub(&mut commands, &asset_server, &root_q, *screen, timer_idx.0, &session, &loaded_stats.0);
        }
    }
}

fn handle_pvl_difficulty(
    q: Query<(&Interaction, &BtnPvLDifficulty), Changed<Interaction>>,
    mut config: ResMut<GameConfig>,
    mut screen: ResMut<PvLHubScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<PvLHubRoot>>,
    asset_server: Res<AssetServer>,
    timer_idx: Res<PvLTimerIdx>,
    session: Res<UserSession>,
    loaded_stats: Res<LoadedStats>,
) {
    for (i, btn) in &q {
        if *i == Interaction::Pressed {
            config.difficulty = btn.0;
            *screen = PvLHubScreen::TimerSelect;
            rebuild_pvl_hub(&mut commands, &asset_server, &root_q, *screen, timer_idx.0, &session, &loaded_stats.0);
        }
    }
}

fn handle_pvl_timer_down(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnPvLTimerDown>)>,
    mut timer_idx: ResMut<PvLTimerIdx>,
    mut commands: Commands,
    root_q: Query<Entity, With<PvLHubRoot>>,
    asset_server: Res<AssetServer>,
    session: Res<UserSession>,
    loaded_stats: Res<LoadedStats>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            timer_idx.0 = if timer_idx.0 == 0 { TIMER_PRESETS_MINS.len() - 1 } else { timer_idx.0 - 1 };
            let idx = timer_idx.0;
            rebuild_pvl_hub(&mut commands, &asset_server, &root_q, PvLHubScreen::TimerSelect, idx, &session, &loaded_stats.0);
        }
    }
}

fn handle_pvl_timer_up(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnPvLTimerUp>)>,
    mut timer_idx: ResMut<PvLTimerIdx>,
    mut commands: Commands,
    root_q: Query<Entity, With<PvLHubRoot>>,
    asset_server: Res<AssetServer>,
    session: Res<UserSession>,
    loaded_stats: Res<LoadedStats>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            timer_idx.0 = (timer_idx.0 + 1) % TIMER_PRESETS_MINS.len();
            let idx = timer_idx.0;
            rebuild_pvl_hub(&mut commands, &asset_server, &root_q, PvLHubScreen::TimerSelect, idx, &session, &loaded_stats.0);
        }
    }
}

fn handle_pvl_no_timer(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnPvLNoTimer>)>,
    mut config: ResMut<GameConfig>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            config.timer_secs = None;
            next_state.set(AppState::Playing);
        }
    }
}

fn handle_pvl_play(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnPvLPlay>)>,
    mut config: ResMut<GameConfig>,
    timer_idx: Res<PvLTimerIdx>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            config.timer_secs = Some(TIMER_PRESETS_MINS[timer_idx.0] * 60);
            next_state.set(AppState::Playing);
        }
    }
}

fn handle_pvl_back(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnPvLBack>)>,
    mut screen: ResMut<PvLHubScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<PvLHubRoot>>,
    asset_server: Res<AssetServer>,
    timer_idx: Res<PvLTimerIdx>,
    session: Res<UserSession>,
    loaded_stats: Res<LoadedStats>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            match *screen {
                PvLHubScreen::Hub | PvLHubScreen::NeedLogin => {
                    next_state.set(AppState::Home);
                }
                PvLHubScreen::ColorSelect => {
                    *screen = PvLHubScreen::Hub;
                    rebuild_pvl_hub(&mut commands, &asset_server, &root_q, *screen, timer_idx.0, &session, &loaded_stats.0);
                }
                PvLHubScreen::DifficultySelect => {
                    *screen = PvLHubScreen::ColorSelect;
                    rebuild_pvl_hub(&mut commands, &asset_server, &root_q, *screen, timer_idx.0, &session, &loaded_stats.0);
                }
                PvLHubScreen::TimerSelect => {
                    *screen = PvLHubScreen::DifficultySelect;
                    rebuild_pvl_hub(&mut commands, &asset_server, &root_q, *screen, timer_idx.0, &session, &loaded_stats.0);
                }
            }
        }
    }
}

// ─── Stats fetch (WASM only) ──────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
async fn fetch_stats_async(jwt: String) -> Option<FetchedStats> {
    #[derive(serde::Deserialize)]
    struct Resp {
        wins:               i64,
        losses:             i64,
        draws:              i64,
        accuracy_avg:       Option<f32>,
        blunders_total:     i64,
        mistakes_total:     i64,
        inaccuracies_total: i64,
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
    })
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct PvLHubPlugin;

impl Plugin for PvLHubPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PvLHubScreen>()
            .init_resource::<PvLTimerIdx>()
            .init_resource::<StatsFetchState>()
            .init_resource::<LoadedStats>()
            .add_systems(OnEnter(AppState::PvLHub), setup_pvl_hub)
            .add_systems(OnExit(AppState::PvLHub),  despawn_pvl_hub)
            .add_systems(Update, (
                highlight_pvl_buttons,
                poll_stats_result,
                handle_pvl_jugar,
                handle_pvl_oauth,
                handle_pvl_color_select,
                handle_pvl_difficulty,
                handle_pvl_timer_down,
                handle_pvl_timer_up,
                handle_pvl_no_timer,
                handle_pvl_play,
                handle_pvl_back,
            ).run_if(in_state(AppState::PvLHub)));
    }
}
