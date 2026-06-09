use bevy::prelude::*;
use crate::board::PlayerTurn;
use crate::pieces::PieceColor;
use crate::ai::Difficulty;

#[derive(Component)]
struct NextMoveText;

#[derive(Component)]
struct DifficultyText;

fn init_ui(mut commands: Commands, asset_server: Res<AssetServer>, difficulty: Res<Difficulty>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "Proximo en mover: Blancas",
                    TextStyle {
                        font: font.clone(),
                        font_size: 40.0,
                        color: Color::rgb(0.8, 0.8, 0.8),
                    },
                ),
                NextMoveText,
            ));
            parent.spawn((
                TextBundle::from_section(
                    format!("Dificultad: {} (Tab para cambiar)", difficulty.label()),
                    TextStyle {
                        font,
                        font_size: 28.0,
                        color: Color::rgb(0.6, 0.8, 0.6),
                    },
                ),
                DifficultyText,
            ));
        });
}

fn next_move_text_update(
    turn: Res<PlayerTurn>,
    mut query: Query<&mut Text, With<NextMoveText>>,
) {
    if !turn.is_changed() {
        return;
    }
    for mut text in query.iter_mut() {
        text.sections[0].value = format!(
            "Proximo en mover: {}",
            match turn.0 {
                PieceColor::White => "Blancas",
                PieceColor::Black => "Negras (IA)",
            }
        );
    }
}

fn cycle_difficulty(
    keys: Res<ButtonInput<KeyCode>>,
    mut difficulty: ResMut<Difficulty>,
    mut query: Query<&mut Text, With<DifficultyText>>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        *difficulty = difficulty.next();
        for mut text in query.iter_mut() {
            text.sections[0].value =
                format!("Dificultad: {} (Tab para cambiar)", difficulty.label());
        }
    }
}

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_ui)
            .add_systems(Update, (next_move_text_update, cycle_difficulty));
    }
}
