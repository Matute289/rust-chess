use bevy::prelude::*;
use crate::ai::Difficulty;
use crate::state::{AppState, GameConfig, GameMode, PvLMode};

// ─── Constants ────────────────────────────────────────────────────────────────

const TIMER_PRESETS_MINS: &[u32] = &[
    1, 3, 5, 10, 15, 20, 25, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120,
];

// ─── User menu ───────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub(crate) struct UserMenuOpen(bool);

#[derive(Component)] pub(crate) struct UserMenuRoot;
#[derive(Component)] struct BtnUserMenu;
#[derive(Component)] struct BtnLogout;

fn build_user_menu(
    commands: &mut Commands,
    asset_server: &AssetServer,
    session: &crate::auth::UserSession,
    open: bool,
) {
    if !session.is_logged_in() { return; }

    let font: Handle<Font> = asset_server.load("fonts/DejaVuSans-Bold.ttf");
    let name = session.display_name.clone().unwrap_or_else(|| "Usuario".to_string());
    let chevron = if open { "icons/chevron-up.png" } else { "icons/chevron-down.png" };

    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                right: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            z_index: ZIndex::Global(10),
            ..default()
        },
        UserMenuRoot,
    ))
    .with_children(|root| {
        // chip button
        root.spawn((
            ButtonBundle {
                style: Style {
                    padding: UiRect {
                        left: Val::Px(16.0), right: Val::Px(16.0),
                        top: Val::Px(8.0),  bottom: Val::Px(8.0),
                    },
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.15, 0.15, 0.28, 0.92)),
                border_color: BorderColor(Color::rgba(0.5, 0.5, 0.75, 0.6)),
                ..default()
            },
            BtnUserMenu,
        ))
        .with_children(|p| {
            p.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|row| {
                row.spawn(TextBundle::from_section(
                    name.clone(),
                    TextStyle { font: font.clone(), font_size: 20.0, color: Color::rgb(0.92, 0.92, 0.92) },
                ));
                row.spawn(ImageBundle {
                    style: Style { width: Val::Px(18.0), height: Val::Px(18.0), ..default() },
                    image: UiImage::new(asset_server.load(chevron)),
                    ..default()
                });
            });
        });

        if open {
            root.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    min_width: Val::Px(200.0),
                    border: UiRect::all(Val::Px(1.0)),
                    margin: UiRect { top: Val::Px(4.0), ..default() },
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.10, 0.10, 0.20, 0.97)),
                border_color: BorderColor(Color::rgba(0.5, 0.5, 0.75, 0.6)),
                ..default()
            })
            .with_children(|panel| {
                panel.spawn((
                    ButtonBundle {
                        style: Style {
                            padding: UiRect {
                                left: Val::Px(16.0), right: Val::Px(16.0),
                                top: Val::Px(12.0),  bottom: Val::Px(12.0),
                            },
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.0)),
                        ..default()
                    },
                    BtnLogout,
                ))
                .with_children(|p| {
                    p.spawn(TextBundle::from_section(
                        "Cerrar sesión",
                        TextStyle { font, font_size: 20.0, color: Color::rgb(0.95, 0.45, 0.45) },
                    ));
                });
            });
        }
    });
}

pub fn spawn_user_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    session: Res<crate::auth::UserSession>,
    menu_open: Res<UserMenuOpen>,
) {
    build_user_menu(&mut commands, &asset_server, &session, menu_open.0);
}

pub fn despawn_user_menu(mut commands: Commands, q: Query<Entity, With<UserMenuRoot>>) {
    for e in &q { commands.entity(e).despawn_recursive(); }
}

fn handle_user_menu_btn(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnUserMenu>)>,
    mut menu_open: ResMut<UserMenuOpen>,
    mut commands: Commands,
    menu_q: Query<Entity, With<UserMenuRoot>>,
    asset_server: Res<AssetServer>,
    session: Res<crate::auth::UserSession>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            menu_open.0 = !menu_open.0;
            for e in &menu_q { commands.entity(e).despawn_recursive(); }
            build_user_menu(&mut commands, &asset_server, &session, menu_open.0);
        }
    }
}

fn handle_logout(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnLogout>)>,
    mut session: ResMut<crate::auth::UserSession>,
    mut menu_open: ResMut<UserMenuOpen>,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut home_screen: ResMut<HomeScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<HomeRoot>>,
    asset_server: Res<AssetServer>,
    timer_idx: Res<SelectedTimerIdx>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            *session = crate::auth::UserSession::default();
            menu_open.0 = false;
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(win) = web_sys::window() {
                    if let Ok(Some(storage)) = win.local_storage() {
                        let _ = storage.remove_item("chess_jwt");
                    }
                }
            }
            if *current_state.get() == AppState::Home {
                *home_screen = HomeScreen::ModeSelect;
                rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::ModeSelect, timer_idx.0);
            } else {
                next_state.set(AppState::Home);
            }
        }
    }
}

fn refresh_user_menu_on_session_change(
    session: Res<crate::auth::UserSession>,
    menu_open: Res<UserMenuOpen>,
    mut commands: Commands,
    menu_q: Query<Entity, With<UserMenuRoot>>,
    asset_server: Res<AssetServer>,
) {
    if !session.is_changed() { return; }
    for e in &menu_q { commands.entity(e).despawn_recursive(); }
    build_user_menu(&mut commands, &asset_server, &session, menu_open.0);
}

// ─── Screen state ─────────────────────────────────────────────────────────────

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum HomeScreen {
    #[default]
    ModeSelect,
    LoginPrompt,
    ColorSelect,
    DifficultySelect,
    TimerSelect,
}

/// Set this before transitioning to AppState::Home to start on a specific sub-screen.
/// Consumed once by reset_home_screen and then reset to ModeSelect.
#[derive(Resource, Default)]
pub struct HomeEntryScreen(pub HomeScreen);

#[derive(Resource)]
struct SelectedTimerIdx(usize);
impl Default for SelectedTimerIdx {
    fn default() -> Self { Self(2) } // 5 min
}

// ─── Components ──────────────────────────────────────────────────────────────

#[derive(Component)] struct HomeRoot;
#[derive(Component)] struct BtnPvP;
#[derive(Component)] struct BtnPvC;
#[derive(Component)] struct BtnPvL;
#[derive(Component)] struct BtnDifficulty(pub Difficulty);
#[derive(Component)] struct BtnOAuth(pub &'static str);
#[derive(Component)] struct BtnBack;
#[derive(Component)] struct BtnSideWhite;
#[derive(Component)] struct BtnSideBlack;
#[derive(Component)] struct BtnTimerDown;
#[derive(Component)] struct BtnTimerUp;
#[derive(Component)] struct BtnNoTimer;
#[derive(Component)] struct BtnPlay;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_btn(
    parent: &mut ChildBuilder,
    font: Handle<Font>,
    text: &str,
    marker: impl Bundle,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(360.0),
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
                TextStyle {
                    font,
                    font_size: 28.0,
                    color: Color::rgb(0.92, 0.92, 0.92),
                },
            ));
        });
}

fn spacer(parent: &mut ChildBuilder, px: f32) {
    parent.spawn(NodeBundle {
        style: Style { height: Val::Px(px), ..default() },
        ..default()
    });
}

// ─── Spawn the home screen ───────────────────────────────────────────────────

fn spawn_home(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    home_screen: Res<HomeScreen>,
    timer_idx: Res<SelectedTimerIdx>,
) {
    build_home_root(&mut commands, &asset_server, *home_screen, timer_idx.0);
}

fn build_home_root(
    commands: &mut Commands,
    asset_server: &AssetServer,
    screen: HomeScreen,
    timer_idx: usize,
) {
    let font: Handle<Font> = asset_server.load("fonts/DejaVuSans-Bold.ttf");

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
            HomeRoot,
        ))
        .with_children(|root| {
            root.spawn(TextBundle::from_section(
                "AJEDREZ",
                TextStyle { font: font.clone(), font_size: 68.0, color: Color::rgb(0.95, 0.92, 0.80) },
            ));
            spacer(root, 40.0);

            match screen {
                HomeScreen::ModeSelect => {
                    root.spawn(TextBundle::from_section(
                        "Seleccioná el modo de juego",
                        TextStyle { font: font.clone(), font_size: 30.0, color: Color::rgb(0.7, 0.7, 0.85) },
                    ));
                    spacer(root, 24.0);
                    make_btn(root, font.clone(), "Player VS Player",   BtnPvP);
                    make_btn(root, font.clone(), "Player VS Computer", BtnPvC);
                    make_btn(root, font.clone(), "Player VS Learning", BtnPvL);
                    spacer(root, 28.0);
                    root.spawn((
                        ButtonBundle {
                            style: Style {
                                padding: UiRect { left: Val::Px(10.0), right: Val::Px(10.0), top: Val::Px(4.0), bottom: Val::Px(4.0) },
                                ..default()
                            },
                            background_color: BackgroundColor(Color::NONE),
                            ..default()
                        },
                        crate::feedback_ui::BtnFeedback,
                    ))
                    .with_children(|p| {
                        p.spawn(TextBundle::from_section(
                            "¿Bugs o sugerencias? Reportar →",
                            TextStyle { font: font.clone(), font_size: 13.0, color: Color::rgba(0.42, 0.42, 0.60, 0.80) },
                        ));
                    });
                }
                HomeScreen::LoginPrompt => {
                    root.spawn(TextBundle::from_section(
                        "Iniciar sesión",
                        TextStyle { font: font.clone(), font_size: 40.0, color: Color::rgb(0.95, 0.92, 0.80) },
                    ));
                    spacer(root, 8.0);
                    root.spawn(TextBundle::from_section(
                        "Elige cómo acceder a tu cuenta:",
                        TextStyle { font: font.clone(), font_size: 22.0, color: Color::rgb(0.65, 0.65, 0.80) },
                    ));
                    spacer(root, 28.0);
                    make_btn(root, font.clone(), "Continuar con Google",  BtnOAuth("Google"));
                    make_btn(root, font.clone(), "Continuar con Apple",   BtnOAuth("Apple"));
                    make_btn(root, font.clone(), "Continuar con GitHub",  BtnOAuth("GitHub"));
                    make_btn(root, font.clone(), "Continuar con Discord", BtnOAuth("Discord"));
                    spacer(root, 16.0);
                    make_btn(root, font.clone(), "← Volver", BtnBack);
                }
                HomeScreen::ColorSelect => {
                    root.spawn(TextBundle::from_section(
                        "¿Con qué color jugás?",
                        TextStyle { font: font.clone(), font_size: 36.0, color: Color::rgb(0.7, 0.7, 0.85) },
                    ));
                    spacer(root, 24.0);
                    make_btn(root, font.clone(), "Blancas", BtnSideWhite);
                    make_btn(root, font.clone(), "Negras",  BtnSideBlack);
                    spacer(root, 12.0);
                    make_btn(root, font.clone(), "← Volver", BtnBack);
                }
                HomeScreen::DifficultySelect => {
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
                        make_btn(root, font.clone(), d.label(), BtnDifficulty(d));
                    }
                    spacer(root, 12.0);
                    make_btn(root, font.clone(), "← Volver", BtnBack);
                }
                HomeScreen::TimerSelect => {
                    root.spawn(TextBundle::from_section(
                        "¿Cuánto tiempo por turno?",
                        TextStyle { font: font.clone(), font_size: 32.0, color: Color::rgb(0.7, 0.7, 0.85) },
                    ));
                    spacer(root, 28.0);

                    // Selector row: [<]  X min  [>]
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
                            BtnTimerDown,
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
                            BtnTimerUp,
                        ))
                        .with_children(|p| {
                            p.spawn(TextBundle::from_section(">", TextStyle {
                                font: font.clone(), font_size: 32.0, color: Color::rgb(0.92, 0.92, 0.92),
                            }));
                        });
                    });

                    spacer(root, 20.0);
                    make_btn(root, font.clone(), "Sin reloj", BtnNoTimer);
                    make_btn(root, font.clone(), "Jugar",     BtnPlay);
                    spacer(root, 8.0);
                    make_btn(root, font.clone(), "← Volver",  BtnBack);
                }
            }
        });
}

fn despawn_home(mut commands: Commands, q: Query<Entity, With<HomeRoot>>) {
    for e in &q { commands.entity(e).despawn_recursive(); }
}

fn rebuild_home(
    commands: &mut Commands,
    asset_server: &AssetServer,
    root_q: &Query<Entity, With<HomeRoot>>,
    screen: HomeScreen,
    timer_idx: usize,
) {
    for e in root_q { commands.entity(e).despawn_recursive(); }
    build_home_root(commands, asset_server, screen, timer_idx);
}

// ─── Button hover highlight ───────────────────────────────────────────────────

fn highlight_buttons(
    mut q: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, mut color) in &mut q {
        *color = match interaction {
            Interaction::Pressed  => BackgroundColor(Color::rgba(0.28, 0.28, 0.55, 0.97)),
            Interaction::Hovered  => BackgroundColor(Color::rgba(0.22, 0.22, 0.45, 0.95)),
            Interaction::None     => BackgroundColor(Color::rgba(0.15, 0.15, 0.28, 0.92)),
        };
    }
}

// ─── Mode select handlers ─────────────────────────────────────────────────────

fn handle_pvp(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnPvP>)>,
    mut config: ResMut<GameConfig>,
    mut home_screen: ResMut<HomeScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<HomeRoot>>,
    asset_server: Res<AssetServer>,
    timer_idx: Res<SelectedTimerIdx>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            config.mode = GameMode::PvP;
            *home_screen = HomeScreen::TimerSelect;
            rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::TimerSelect, timer_idx.0);
        }
    }
}

fn handle_pvc(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnPvC>)>,
    mut config: ResMut<GameConfig>,
    mut home_screen: ResMut<HomeScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<HomeRoot>>,
    asset_server: Res<AssetServer>,
    timer_idx: Res<SelectedTimerIdx>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            config.mode = GameMode::PvC;
            *home_screen = HomeScreen::ColorSelect;
            rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::ColorSelect, timer_idx.0);
        }
    }
}

fn handle_pvl(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnPvL>)>,
    mut config: ResMut<GameConfig>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            config.mode = GameMode::PvL;
            next_state.set(AppState::PvLHub);
        }
    }
}

// ─── OAuth login (stub) ───────────────────────────────────────────────────────

fn handle_oauth(
    q: Query<(&Interaction, &BtnOAuth), Changed<Interaction>>,
    mut home_screen: ResMut<HomeScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<HomeRoot>>,
    asset_server: Res<AssetServer>,
    timer_idx: Res<SelectedTimerIdx>,
) {
    for (interaction, btn) in &q {
        if *interaction == Interaction::Pressed {
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
                return;
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                *home_screen = HomeScreen::ColorSelect;
                rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::ColorSelect, timer_idx.0);
            }
        }
    }
}

// ─── Color select ────────────────────────────────────────────────────────────

fn handle_color_select(
    white_q: Query<&Interaction, (Changed<Interaction>, With<BtnSideWhite>)>,
    black_q: Query<&Interaction, (Changed<Interaction>, With<BtnSideBlack>)>,
    mut config:      ResMut<GameConfig>,
    mut home_screen: ResMut<HomeScreen>,
    mut commands:    Commands,
    root_q:          Query<Entity, With<HomeRoot>>,
    asset_server:    Res<AssetServer>,
    timer_idx:       Res<SelectedTimerIdx>,
    mut next_state:  ResMut<NextState<AppState>>,
) {
    let adaptativa = config.mode == GameMode::PvL && config.pvl_mode == PvLMode::Adaptativa;

    for i in &white_q {
        if *i == Interaction::Pressed {
            config.player_side = crate::pieces::PieceColor::White;
            if adaptativa {
                next_state.set(AppState::Playing);
            } else {
                *home_screen = HomeScreen::DifficultySelect;
                rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::DifficultySelect, timer_idx.0);
            }
        }
    }
    for i in &black_q {
        if *i == Interaction::Pressed {
            config.player_side = crate::pieces::PieceColor::Black;
            if adaptativa {
                next_state.set(AppState::Playing);
            } else {
                *home_screen = HomeScreen::DifficultySelect;
                rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::DifficultySelect, timer_idx.0);
            }
        }
    }
}

// ─── Difficulty select ────────────────────────────────────────────────────────

fn handle_difficulty(
    q: Query<(&Interaction, &BtnDifficulty), Changed<Interaction>>,
    mut config: ResMut<GameConfig>,
    mut home_screen: ResMut<HomeScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<HomeRoot>>,
    asset_server: Res<AssetServer>,
    timer_idx: Res<SelectedTimerIdx>,
) {
    for (i, btn) in &q {
        if *i == Interaction::Pressed {
            config.difficulty = btn.0;
            *home_screen = HomeScreen::TimerSelect;
            rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::TimerSelect, timer_idx.0);
        }
    }
}

// ─── Timer select ─────────────────────────────────────────────────────────────

fn handle_timer_down(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnTimerDown>)>,
    mut timer_idx: ResMut<SelectedTimerIdx>,
    mut commands: Commands,
    root_q: Query<Entity, With<HomeRoot>>,
    asset_server: Res<AssetServer>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            timer_idx.0 = if timer_idx.0 == 0 { TIMER_PRESETS_MINS.len() - 1 } else { timer_idx.0 - 1 };
            let idx = timer_idx.0;
            rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::TimerSelect, idx);
        }
    }
}

fn handle_timer_up(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnTimerUp>)>,
    mut timer_idx: ResMut<SelectedTimerIdx>,
    mut commands: Commands,
    root_q: Query<Entity, With<HomeRoot>>,
    asset_server: Res<AssetServer>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            timer_idx.0 = (timer_idx.0 + 1) % TIMER_PRESETS_MINS.len();
            let idx = timer_idx.0;
            rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::TimerSelect, idx);
        }
    }
}

fn handle_no_timer(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnNoTimer>)>,
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

fn handle_play(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnPlay>)>,
    mut config: ResMut<GameConfig>,
    timer_idx: Res<SelectedTimerIdx>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            config.timer_secs = Some(TIMER_PRESETS_MINS[timer_idx.0] * 60);
            next_state.set(AppState::Playing);
        }
    }
}

// ─── Back button ──────────────────────────────────────────────────────────────

fn handle_back(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnBack>)>,
    config: Res<GameConfig>,
    mut home_screen: ResMut<HomeScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<HomeRoot>>,
    asset_server: Res<AssetServer>,
    timer_idx: Res<SelectedTimerIdx>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            // ColorSelect reached from PvLHub → go back to the hub
            if *home_screen == HomeScreen::ColorSelect && config.mode == GameMode::PvL {
                next_state.set(AppState::PvLHub);
                return;
            }
            let target = match *home_screen {
                HomeScreen::TimerSelect => {
                    if config.mode == GameMode::PvP { HomeScreen::ModeSelect }
                    else { HomeScreen::DifficultySelect }
                }
                _ => HomeScreen::ModeSelect,
            };
            *home_screen = target;
            rebuild_home(&mut commands, &asset_server, &root_q, target, timer_idx.0);
        }
    }
}

// ─── Reset HomeScreen on re-enter ─────────────────────────────────────────────

fn reset_home_screen(
    mut home_screen: ResMut<HomeScreen>,
    mut timer_idx: ResMut<SelectedTimerIdx>,
    mut menu_open: ResMut<UserMenuOpen>,
    mut entry: ResMut<HomeEntryScreen>,
) {
    *home_screen = entry.0;
    entry.0 = HomeScreen::ModeSelect; // consume — reset for next entry
    *timer_idx = SelectedTimerIdx::default();
    menu_open.0 = false;
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct HomePlugin;

impl Plugin for HomePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<HomeScreen>()
            .init_resource::<HomeEntryScreen>()
            .init_resource::<SelectedTimerIdx>()
            .init_resource::<UserMenuOpen>()
            .add_systems(OnEnter(AppState::Home), (reset_home_screen, spawn_home, spawn_user_menu).chain())
            .add_systems(OnExit(AppState::Home),  (despawn_home, despawn_user_menu))
            .add_systems(Update, (
                highlight_buttons,
                handle_pvp,
                handle_pvc,
                handle_pvl,
                handle_oauth,
                handle_color_select,
                handle_difficulty,
                handle_timer_down,
                handle_timer_up,
                handle_no_timer,
                handle_play,
                handle_back,
            ).run_if(in_state(AppState::Home)))
            .add_systems(Update, (
                handle_user_menu_btn,
                handle_logout,
                refresh_user_menu_on_session_change,
            ).run_if(in_state(AppState::Home).or_else(in_state(AppState::PvLHub))));
    }
}

#[cfg(test)]
mod tests {
    use crate::ai::Difficulty;

    #[test]
    fn difficulty_labels_are_non_empty() {
        let all = [
            Difficulty::Principiante, Difficulty::Facil, Difficulty::Medio,
            Difficulty::Dificil, Difficulty::Pro,
        ];
        for d in all {
            assert!(!d.label().is_empty());
        }
    }
}
