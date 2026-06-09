use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Home,
    Playing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameMode {
    #[default]
    PvP,
    PvC,
    PvL,
}

#[derive(Resource, Default)]
pub struct GameConfig {
    pub mode: GameMode,
    pub difficulty: crate::ai::Difficulty,
}
