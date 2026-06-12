use bevy::prelude::*;
use chess_engine::{GameRecord, GameReport, GameResult as EngineGameResult, MoveAnalysis, MoveClass, analyze_game};
use crate::board::{GameHistory, GameStatus, GameStatusEvent};
use crate::pieces::PieceColor;
use crate::state::{AppState, GameConfig, GameMode};

// ─── Resources ───────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct AnalysisReport(pub Option<GameReport>);

#[derive(Resource, Default)]
pub struct GameNarrative(pub Option<String>);

// ─── Narrative generation ─────────────────────────────────────────────────────

fn piece_char_at(fen: &str, file: char, rank: char) -> char {
    let file_idx = file as usize - 'a' as usize;
    let rank_idx = '8' as usize - rank as usize;
    let board = fen.split(' ').next().unwrap_or("");
    let (mut row, mut col) = (0usize, 0usize);
    for c in board.chars() {
        if c == '/' { row += 1; col = 0; continue; }
        if let Some(n) = c.to_digit(10) { col += n as usize; continue; }
        if row == rank_idx && col == file_idx { return c; }
        col += 1;
    }
    '?'
}

fn describe_move(uci: &str, fen: &str) -> (&'static str, String, String) {
    let b = uci.as_bytes();
    if b.len() < 4 { return ("pieza", "??".to_string(), "??".to_string()); }
    let from = format!("{}{}", b[0] as char, b[1] as char);
    let to   = format!("{}{}", b[2] as char, b[3] as char);
    let piece = piece_char_at(fen, b[0] as char, b[1] as char);
    let name: &'static str = match piece.to_ascii_lowercase() {
        'p' => "peón", 'n' => "caballo", 'b' => "alfil",
        'r' => "torre", 'q' => "dama",   'k' => "rey",
        _   => "pieza",
    };
    (name, from, to)
}

pub fn generate_narrative(report: &GameReport, player_side: PieceColor) -> String {
    let s  = &report.summary;
    let si = match player_side { PieceColor::White => 0, PieceColor::Black => 1 };

    let acc    = if si == 0 { s.accuracy_white } else { s.accuracy_black };
    let nblnd  = s.blunders[si];
    let nmist  = s.mistakes[si];
    let ninacc = s.inaccuracies[si];

    // Player's moves: white = even indices (0,2,4...), black = odd (1,3,5...)
    let my: Vec<(usize, &MoveAnalysis)> = report.move_analyses.iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == si)
        .collect();

    let mut parts: Vec<String> = Vec::new();

    // 1. Overall quality
    let quality = if acc >= 90.0 { "¡Partida excelente" }
                  else if acc >= 75.0 { "Partida sólida" }
                  else if acc >= 55.0 { "Partida con errores" }
                  else { "Partida muy difícil" };
    parts.push(format!("{} ({:.0}% de precisión).", quality, acc));

    // 2. Opening phase (first 5 player moves)
    let op_hard = my.iter().take(5)
        .filter(|(_, a)| matches!(a.classification, MoveClass::Blunder | MoveClass::Mistake))
        .count();
    let op_soft = my.iter().take(5)
        .filter(|(_, a)| matches!(a.classification, MoveClass::Inaccuracy))
        .count();

    parts.push(if op_hard > 0 {
        "Hubo problemas en la apertura.".to_string()
    } else if op_soft > 1 {
        "La apertura tuvo algunas imprecisiones.".to_string()
    } else {
        "La apertura estuvo bien.".to_string()
    });

    // 3. Critical mistakes (up to 2)
    for &(abs_i, a) in my.iter()
        .filter(|(_, a)| matches!(a.classification, MoveClass::Blunder | MoveClass::Mistake))
        .take(2)
    {
        let move_num = abs_i / 2 + 1;
        let (pname, from_sq, to_sq) = describe_move(&a.played_move, &a.fen_before);
        let label = if matches!(a.classification, MoveClass::Blunder) { "error grave" } else { "error" };

        let sentence = if a.best_move != a.played_move && a.best_move.len() >= 4 {
            let (_, bfrom, bto) = describe_move(&a.best_move, &a.fen_before);
            format!("Movimiento {}: {} con el {} ({}→{}); era mejor {}→{}.",
                move_num, label, pname, from_sq, to_sq, bfrom, bto)
        } else {
            format!("Movimiento {}: {} con el {} ({}→{}).",
                move_num, label, pname, from_sq, to_sq)
        };
        parts.push(sentence);
    }

    // 4. Inaccuracy advice
    if ninacc >= 5 {
        parts.push("Muchas imprecisiones — intentá calcular más antes de mover.".to_string());
    } else if ninacc >= 3 {
        parts.push("Hubo varias imprecisiones menores que se pueden mejorar.".to_string());
    }

    // 5. Praise or advice
    if nblnd == 0 && nmist == 0 && !my.is_empty() {
        parts.push("¡Sin errores graves ni errores en esta partida!".to_string());
    } else if nblnd == 0 && !my.is_empty() {
        parts.push("Lo positivo: sin errores graves esta partida.".to_string());
    }

    parts.join(" ")
}

// ─── Systems ─────────────────────────────────────────────────────────────────

pub fn run_post_game_analysis(
    mut events:     EventReader<GameStatusEvent>,
    history:        Res<GameHistory>,
    config:         Res<GameConfig>,
    mut report:     ResMut<AnalysisReport>,
    mut narrative:  ResMut<GameNarrative>,
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

        let r = analyze_game(&record);

        // Generate colloquial narrative for single-player modes
        if matches!(config.mode, GameMode::PvC | GameMode::PvL) {
            narrative.0 = Some(generate_narrative(&r, config.player_side));
        }

        report.0 = Some(r);
    }
}

fn reset_analysis(mut report: ResMut<AnalysisReport>, mut narrative: ResMut<GameNarrative>) {
    report.0    = None;
    narrative.0 = None;
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct AnalysisPlugin;

impl Plugin for AnalysisPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<AnalysisReport>()
            .init_resource::<GameNarrative>()
            .add_systems(OnEnter(AppState::Playing), reset_analysis)
            .add_systems(First,
                run_post_game_analysis.run_if(in_state(AppState::Playing))
            );
    }
}
