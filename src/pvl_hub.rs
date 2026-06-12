use bevy::prelude::*;
use std::sync::{Arc, Mutex};
use crate::{
    auth::UserSession,
    home::{HomeEntryScreen, HomeScreen},
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
        });
}

fn rebuild_pvl_hub(
    commands: &mut Commands,
    asset_server: &AssetServer,
    root_q: &Query<Entity, With<PvLHubRoot>>,
    screen: PvLHubScreen,
    session: &UserSession,
    stats: &Option<FetchedStats>,
) {
    for e in root_q { commands.entity(e).despawn_recursive(); }
    build_pvl_hub_root(commands, asset_server, screen, session, stats);
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
            .init_resource::<StatsFetchState>()
            .init_resource::<LoadedStats>()
            .add_systems(OnEnter(AppState::PvLHub), setup_pvl_hub)
            .add_systems(OnExit(AppState::PvLHub),  despawn_pvl_hub)
            .add_systems(Update, (
                highlight_pvl_buttons,
                poll_stats_result,
                handle_pvl_jugar,
                handle_pvl_oauth,
                handle_pvl_back,
            ).run_if(in_state(AppState::PvLHub)));
    }
}
