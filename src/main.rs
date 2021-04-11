mod pieces;
mod board;
mod ui;

use bevy::prelude::*;
use bevy_mod_picking::*;
use crate::pieces::{PiecesPlugin};
use crate::board::*;
use crate::ui::UIPlugin;

fn main() {
    App::build()
        .add_resource(Msaa{samples:4})
        .add_resource(WindowDescriptor {
            title: "Ajedrez".to_string(),
            width: 1200.,
            height: 1000.,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(PickingPlugin)
        .add_plugin(BoardPlugin)
        .add_plugin(PiecesPlugin)
        .add_plugin(UIPlugin)
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    commands: &mut Commands,
){
    commands
    // Camara
        .spawn(Camera3dBundle {
            transform: Transform::from_matrix(Mat4::from_rotation_translation(
                Quat::from_xyzw(-0.3,-0.5,-0.3,0.5).normalize(),
                Vec3::new(-4.0, 12.0,4.0),
            )),
            ..Default::default()
        })
        .with(PickSource::default())
    // Luces
        .spawn(LightBundle{
            transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
            ..Default::default()
        });
}