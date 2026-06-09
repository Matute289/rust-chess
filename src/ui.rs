use bevy::prelude::*;
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
            PieceColor::White => "Blancas".to_string(),
            PieceColor::Black => if config.mode == GameMode::PvP {
                "Negras".to_string()
            } else {
                "Negras (Computadora)".to_string()
            },
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
    overlay_q:       Query<Entity, With<GameOverOverlay>>,
    banner_q:        Query<Entity, With<CheckBanner>>,
) {
    for ev in events.read() {
        // Remove any previous check banner
        for e in &banner_q { commands.entity(e).despawn_recursive(); }

        match &ev.0 {
            GameStatus::Check => {
                // Show a temporary check banner
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
                        "  ⚠  ¡JAQUE!  ⚠  ",
                        TextStyle { font, font_size: 48.0, color: Color::rgb(1.0, 0.8, 0.1) },
                    ));
                });
            }
            GameStatus::Checkmate { winner } => {
                for e in &overlay_q { commands.entity(e).despawn_recursive(); }
                let winner_str = match winner {
                    PieceColor::White => "¡Blancas ganan!",
                    PieceColor::Black => "¡Negras ganan!",
                };
                spawn_game_over_overlay(&mut commands, &asset_server, winner_str, false);
            }
            GameStatus::Stalemate => {
                for e in &overlay_q { commands.entity(e).despawn_recursive(); }
                spawn_game_over_overlay(&mut commands, &asset_server, "¡Empate por ahogado!", true);
            }
        }
    }
}

fn spawn_game_over_overlay(commands: &mut Commands, asset_server: &AssetServer, title: &str, is_draw: bool) {
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
                    row_gap: Val::Px(20.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.80)),
                ..default()
            },
            GameOverOverlay,
        ))
        .with_children(|root| {
            root.spawn(TextBundle::from_section(
                title,
                TextStyle { font: font.clone(), font_size: 56.0, color: Color::rgb(1.0, 0.9, 0.2) },
            ));

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
            .add_systems(OnExit(AppState::Playing),  (despawn_hud, despawn_game_over_overlay))
            .add_systems(Update, (
                update_turn_text,
                handle_new_game_btn,
                handle_status_events,
                handle_game_over_buttons,
            ).run_if(in_state(AppState::Playing)));
    }
}
