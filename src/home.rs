use bevy::prelude::*;
use crate::ai::Difficulty;
use crate::state::{AppState, GameConfig, GameMode};

// ─── Screen state ─────────────────────────────────────────────────────────────

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum HomeScreen {
    #[default]
    ModeSelect,
    LoginPrompt,
    ColorSelect,
    DifficultySelect,
}

// ─── Components ──────────────────────────────────────────────────────────────

#[derive(Component)] struct HomeRoot;
#[derive(Component)] struct BtnPvP;
#[derive(Component)] struct BtnPvC;
#[derive(Component)] struct BtnPvL;
#[derive(Component)] struct BtnDifficulty(pub Difficulty);
#[derive(Component)] struct BtnOAuth(pub &'static str); // "Google", "Apple", "GitHub", "Discord"
#[derive(Component)] struct BtnBack;
#[derive(Component)] struct BtnSideWhite;
#[derive(Component)] struct BtnSideBlack;

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
) {
    build_home_root(&mut commands, &asset_server, *home_screen);
}

fn build_home_root(commands: &mut Commands, asset_server: &AssetServer, screen: HomeScreen) {
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
) {
    for e in root_q { commands.entity(e).despawn_recursive(); }
    build_home_root(commands, asset_server, screen);
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
    mut next_state: ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            config.mode = GameMode::PvP;
            next_state.set(AppState::Playing);
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
) {
    for i in &q {
        if *i == Interaction::Pressed {
            config.mode = GameMode::PvC;
            *home_screen = HomeScreen::ColorSelect;
            rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::ColorSelect);
        }
    }
}

fn handle_pvl(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnPvL>)>,
    mut config: ResMut<GameConfig>,
    mut home_screen: ResMut<HomeScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<HomeRoot>>,
    asset_server: Res<AssetServer>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            config.mode = GameMode::PvL;
            *home_screen = HomeScreen::LoginPrompt;
            rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::LoginPrompt);
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
) {
    for (i, _btn) in &q {
        if *i == Interaction::Pressed {
            // Stub: treat any provider click as successful login
            *home_screen = HomeScreen::ColorSelect;
            rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::ColorSelect);
        }
    }
}

// ─── Color select ────────────────────────────────────────────────────────────

fn handle_color_select(
    white_q: Query<&Interaction, (Changed<Interaction>, With<BtnSideWhite>)>,
    black_q: Query<&Interaction, (Changed<Interaction>, With<BtnSideBlack>)>,
    mut config: ResMut<GameConfig>,
    mut home_screen: ResMut<HomeScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<HomeRoot>>,
    asset_server: Res<AssetServer>,
) {
    for i in &white_q {
        if *i == Interaction::Pressed {
            config.player_side = crate::pieces::PieceColor::White;
            *home_screen = HomeScreen::DifficultySelect;
            rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::DifficultySelect);
        }
    }
    for i in &black_q {
        if *i == Interaction::Pressed {
            config.player_side = crate::pieces::PieceColor::Black;
            *home_screen = HomeScreen::DifficultySelect;
            rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::DifficultySelect);
        }
    }
}

// ─── Difficulty select ────────────────────────────────────────────────────────

fn handle_difficulty(
    q: Query<(&Interaction, &BtnDifficulty), Changed<Interaction>>,
    mut config: ResMut<GameConfig>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (i, btn) in &q {
        if *i == Interaction::Pressed {
            config.difficulty = btn.0;
            next_state.set(AppState::Playing);
        }
    }
}

// ─── Back button ──────────────────────────────────────────────────────────────

fn handle_back(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnBack>)>,
    mut home_screen: ResMut<HomeScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<HomeRoot>>,
    asset_server: Res<AssetServer>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            *home_screen = HomeScreen::ModeSelect;
            rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::ModeSelect);
        }
    }
}

// ─── Reset HomeScreen on re-enter ─────────────────────────────────────────────

fn reset_home_screen(mut home_screen: ResMut<HomeScreen>) {
    *home_screen = HomeScreen::ModeSelect;
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct HomePlugin;

impl Plugin for HomePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<HomeScreen>()
            .add_systems(OnEnter(AppState::Home), (reset_home_screen, spawn_home).chain())
            .add_systems(OnExit(AppState::Home),  despawn_home)
            .add_systems(Update, (
                highlight_buttons,
                handle_pvp,
                handle_pvc,
                handle_pvl,
                handle_oauth,
                handle_color_select,
                handle_difficulty,
                handle_back,
            ).run_if(in_state(AppState::Home)));
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
