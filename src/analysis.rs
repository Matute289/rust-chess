use bevy::prelude::*;
use chess_engine::{GameRecord, GameResult as EngineGameResult, GameReport, analyze_game};
use crate::board::{GameHistory, GameStatus, GameStatusEvent};
use crate::pieces::PieceColor;
use crate::state::AppState;

// ─── Resources ───────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct AnalysisReport(pub Option<GameReport>);

// ─── Systems ─────────────────────────────────────────────────────────────────

/// Runs synchronously when the game ends (checkmate or stalemate).
/// Reads GameHistory, calls analyze_game, stores result in AnalysisReport.
/// This frame will stutter by ~1–4s; that's acceptable since the game just ended.
pub fn run_post_game_analysis(
    mut events:  EventReader<GameStatusEvent>,
    history:     Res<GameHistory>,
    mut report:  ResMut<AnalysisReport>,
) {
    for ev in events.read() {
        let engine_result = match &ev.0 {
            GameStatus::Checkmate { winner } => match winner {
                PieceColor::White => EngineGameResult::WhiteWins,
                PieceColor::Black => EngineGameResult::BlackWins,
            },
            GameStatus::Stalemate => EngineGameResult::Draw,
            _ => continue,
        };

        if history.moves.is_empty() { continue; }

        let record = GameRecord {
            initial_fen: history.initial_fen.clone(),
            moves:       history.moves.clone(),
            result:      engine_result,
        };

        report.0 = Some(analyze_game(&record));
    }
}

fn reset_analysis(mut report: ResMut<AnalysisReport>) {
    report.0 = None;
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct AnalysisPlugin;

impl Plugin for AnalysisPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<AnalysisReport>()
            .add_systems(OnEnter(AppState::Playing), reset_analysis)
            .add_systems(First,
                run_post_game_analysis.run_if(in_state(AppState::Playing))
            );
    }
}
