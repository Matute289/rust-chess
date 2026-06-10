use crate::moves::Move;
use crate::position::Position;
use crate::search::{DifficultyConfig, Search, SearchResult};
use crate::types::Color;
use crate::movegen::MoveGen;

// ── Analysis constants ────────────────────────────────────────────────────────

pub const ANALYSIS_DEPTH: u8 = 6;
pub const ANALYSIS_NODES: u64 = 200_000;

// ── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum GameResult {
    WhiteWins,
    BlackWins,
    Draw,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MoveClass {
    Brilliant,
    Excellent,
    Good,
    Inaccuracy,
    Mistake,
    Blunder,
}

#[derive(Debug, Clone)]
pub struct MoveAnalysis {
    pub fen_before:     String,
    pub played_move:    String,   // UCI string e.g. "e2e4"
    pub best_move:      String,   // UCI string
    pub score_after:    i32,      // centipawns from mover's POV after the move (negated from opponent)
    pub best_score:     i32,      // engine's best score for this position
    pub eval_loss:      i32,      // best_score - score_after (≥ 0)
    pub classification: MoveClass,
}

#[derive(Debug, Clone)]
pub struct GameSummary {
    pub accuracy_white:        f32,
    pub accuracy_black:        f32,
    pub blunders:              [u8; 2],   // [white, black]
    pub mistakes:              [u8; 2],
    pub inaccuracies:          [u8; 2],
    pub critical_move_indices: Vec<usize>, // indices into move_analyses where eval_loss > 100cp
}

#[derive(Debug, Clone)]
pub struct GameReport {
    pub move_analyses: Vec<MoveAnalysis>,
    pub summary:       GameSummary,
}

pub struct GameRecord {
    pub initial_fen: String,
    pub moves:       Vec<Move>,
    pub result:      GameResult,
}

// ── Classification ────────────────────────────────────────────────────────────

pub fn classify(eval_loss: i32) -> MoveClass {
    match eval_loss {
        0          => MoveClass::Excellent,
        1..=19     => MoveClass::Good,
        20..=99    => MoveClass::Inaccuracy,
        100..=299  => MoveClass::Mistake,
        _          => MoveClass::Blunder,
    }
}

// ── Accuracy formula ─────────────────────────────────────────────────────────

pub fn compute_accuracy(analyses: &[MoveAnalysis]) -> f32 {
    if analyses.is_empty() { return 100.0; }

    let total: f32 = analyses.iter().map(|a| {
        match a.classification {
            MoveClass::Brilliant | MoveClass::Excellent => 0.0,
            MoveClass::Good       => 2.0,
            MoveClass::Inaccuracy => 8.0,
            MoveClass::Mistake    => 20.0,
            MoveClass::Blunder    => 40.0,
        }
    }).sum();

    let max_penalty = analyses.len() as f32 * 40.0;
    let raw = 1.0 - total / max_penalty;
    (raw * 100.0).max(0.0).min(100.0)
}

// ── Main analysis ─────────────────────────────────────────────────────────────

pub fn analyze_game(record: &GameRecord) -> GameReport {
    let config = DifficultyConfig {
        max_depth: ANALYSIS_DEPTH,
        max_nodes: ANALYSIS_NODES,
        random_factor: 0.0,
    };

    let mut pos = match Position::from_fen(&record.initial_fen) {
        Ok(p) => p,
        Err(_) => return empty_report(),
    };

    let first_mover = pos.side_to_move;
    let mut all_analyses: Vec<MoveAnalysis> = Vec::new();

    for played_move in &record.moves {
        let fen_before = pos.to_fen();
        let mover = pos.side_to_move;

        // Search from this position — gives best_score and best_move
        let result = Search::new().best_move(&pos, &config);
        let (best_move_m, best_score) = match result {
            SearchResult::EngineMove(m, s) => (m, s),
        };

        // Check for Brilliant BEFORE applying the move (need pre-move position)
        let to_sq = played_move.to_sq();
        let opponent = mover.flip();
        let dest_attacked = MoveGen::is_attacked(&pos, to_sq, opponent);

        // Apply played move (SavedState discarded — analysis only walks forward)
        let _ = pos.make_move_mut(*played_move);

        // Evaluate resulting position from opponent's POV, negate for mover
        let result_after = Search::new().best_move(&pos, &config);
        let score_after = match result_after {
            SearchResult::EngineMove(_, s) => -s,
        };

        // eval_loss = how much worse than best? Cap at 0 to handle search noise
        let eval_loss = (best_score - score_after).max(0);

        // Classify, promote to Brilliant if conditions met
        let mut classification = classify(eval_loss);
        if eval_loss == 0 && dest_attacked {
            classification = MoveClass::Brilliant;
        }

        all_analyses.push(MoveAnalysis {
            fen_before,
            played_move:    played_move.to_uci(),
            best_move:      best_move_m.to_uci(),
            score_after,
            best_score,
            eval_loss,
            classification,
        });
    }

    build_report(all_analyses, &record.result, first_mover)
}

fn build_report(analyses: Vec<MoveAnalysis>, _result: &GameResult, first_mover: Color) -> GameReport {
    // Determine which parity of index maps to White vs Black
    // If White moves first (index 0), even indices = White; otherwise even = Black
    let white_is_even = first_mover == Color::White;

    let white_analyses: Vec<MoveAnalysis> = analyses.iter().enumerate()
        .filter(|(i, _)| (i % 2 == 0) == white_is_even).map(|(_, a)| a.clone()).collect();
    let black_analyses: Vec<MoveAnalysis> = analyses.iter().enumerate()
        .filter(|(i, _)| (i % 2 == 0) != white_is_even).map(|(_, a)| a.clone()).collect();

    let accuracy_white = compute_accuracy(&white_analyses);
    let accuracy_black = compute_accuracy(&black_analyses);

    let mut blunders     = [0u8; 2];
    let mut mistakes     = [0u8; 2];
    let mut inaccuracies = [0u8; 2];
    let mut critical     = Vec::new();

    for (i, a) in analyses.iter().enumerate() {
        // side index: 0 = White, 1 = Black
        let side = if (i % 2 == 0) == white_is_even { 0 } else { 1 };
        match a.classification {
            MoveClass::Blunder    => blunders[side]     = blunders[side].saturating_add(1),
            MoveClass::Mistake    => mistakes[side]     = mistakes[side].saturating_add(1),
            MoveClass::Inaccuracy => inaccuracies[side] = inaccuracies[side].saturating_add(1),
            _ => {}
        }
        if matches!(a.classification, MoveClass::Mistake | MoveClass::Blunder) {
            critical.push(i);
        }
    }

    GameReport {
        move_analyses: analyses,
        summary: GameSummary {
            accuracy_white,
            accuracy_black,
            blunders,
            mistakes,
            inaccuracies,
            critical_move_indices: critical,
        },
    }
}

fn empty_report() -> GameReport {
    GameReport {
        move_analyses: Vec::new(),
        summary: GameSummary {
            accuracy_white: 0.0,
            accuracy_black: 0.0,
            blunders:     [0; 2],
            mistakes:     [0; 2],
            inaccuracies: [0; 2],
            critical_move_indices: Vec::new(),
        },
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_class_thresholds() {
        assert_eq!(classify(0),   MoveClass::Excellent);
        assert_eq!(classify(15),  MoveClass::Good);
        assert_eq!(classify(50),  MoveClass::Inaccuracy);
        assert_eq!(classify(150), MoveClass::Mistake);
        assert_eq!(classify(400), MoveClass::Blunder);
    }

    #[test]
    fn accuracy_perfect_game() {
        let analyses = vec![
            MoveAnalysis { eval_loss: 0,  classification: MoveClass::Excellent,
                fen_before: String::new(), played_move: String::new(),
                best_move: String::new(), score_after: 0, best_score: 0 },
            MoveAnalysis { eval_loss: 0,  classification: MoveClass::Excellent,
                fen_before: String::new(), played_move: String::new(),
                best_move: String::new(), score_after: 0, best_score: 0 },
        ];
        let acc = compute_accuracy(&analyses);
        assert!((acc - 100.0).abs() < 0.01, "expected 100.0 got {}", acc);
    }

    #[test]
    fn accuracy_all_blunders() {
        let analyses = vec![
            MoveAnalysis { eval_loss: 500, classification: MoveClass::Blunder,
                fen_before: String::new(), played_move: String::new(),
                best_move: String::new(), score_after: -500, best_score: 0 },
        ];
        let acc = compute_accuracy(&analyses);
        assert!((acc - 0.0).abs() < 0.01, "all-blunder accuracy should be 0.0, got {}", acc);
    }
}
