use bevy::prelude::*;
use crate::pieces::PieceColor;

#[derive(Resource, Default, Clone)]
pub struct Suggestion {
    pub from_sq: Option<(u8, u8)>,
    pub to_sq:   Option<(u8, u8)>,
    pub text:    Option<String>,
}

impl Suggestion {
    pub fn is_active(&self) -> bool { self.from_sq.is_some() }
    pub fn clear(&mut self) { *self = Self::default(); }
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Home,
    PvLHub,
    Playing,
    Lessons,
    LessonRetry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameMode {
    #[default]
    PvP,
    PvC,
    PvL,
    Lesson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PvLMode {
    #[default]
    Standard,
    Adaptativa,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LessonMode {
    #[default]
    Interactive,
    Guided,
}

#[derive(Resource, Default, Clone)]
pub struct LessonSetup {
    pub lesson_idx:  usize,
    pub fen:         String,
    pub answer_uci:  String,
    pub lesson_mode: LessonMode,
    pub title:       String,
    pub description: String,
}

impl LessonSetup {
    pub fn is_last_exercise(&self) -> bool { true }
}

#[derive(Resource)]
pub struct GameConfig {
    pub mode:        GameMode,
    pub pvl_mode:    PvLMode,
    pub difficulty:  crate::ai::Difficulty,
    pub player_side: PieceColor,
    pub timer_secs:  Option<u32>,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            mode:        GameMode::default(),
            pvl_mode:    PvLMode::default(),
            difficulty:  crate::ai::Difficulty::default(),
            player_side: PieceColor::White,
            timer_secs:  None,
        }
    }
}
