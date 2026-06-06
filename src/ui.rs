use bevy::prelude::*;
use crate::board::{PlayerTurn, SelectedSquare};
use crate::pieces::PieceColor;

#[derive(Component)]
struct NextMoveText;

fn init_next_move_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "Proximo en mover: Blancas",
                    TextStyle {
                        font,
                        font_size: 40.0,
                        color: Color::rgb(0.8, 0.8, 0.8),
                    },
                ),
                NextMoveText,
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
                PieceColor::Black => "Negras",
            }
        );
    }
}

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_next_move_text)
            .add_systems(Update, next_move_text_update);
    }
}
