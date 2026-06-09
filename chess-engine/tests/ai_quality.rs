use chess_engine::{DifficultyConfig, Position, Search, SearchResult};

fn best_move_uci(fen: &str, config: &DifficultyConfig) -> String {
    let pos = Position::from_fen(fen).unwrap();
    let SearchResult::EngineMove(m, _) = Search::new().best_move(&pos, config);
    m.to_uci()
}

fn best_move_score(fen: &str, config: &DifficultyConfig) -> i32 {
    let pos = Position::from_fen(fen).unwrap();
    let SearchResult::EngineMove(_, s) = Search::new().best_move(&pos, config);
    s
}

#[test]
fn finds_mate_in_one() {
    assert_eq!(
        best_move_uci("k7/R7/1K6/8/8/8/8/8 w - - 0 1", &DifficultyConfig::facil()),
        "a7a8"
    );
}

#[test]
fn startpos_score_is_balanced() {
    let score = best_move_score(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        &DifficultyConfig::medio(),
    );
    assert!(score.abs() < 300, "startpos score {} is too extreme", score);
}

#[test]
fn recognizes_material_advantage() {
    // White is up a queen — score should be strongly positive
    let score = best_move_score(
        "rnbqkbnr/pppppppp/8/8/8/Q7/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        &DifficultyConfig::medio(),
    );
    assert!(score > 700, "extra queen score {} too low", score);
}

#[test]
fn all_difficulties_return_move() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    for config in [
        DifficultyConfig::principiante(),
        DifficultyConfig::facil(),
        DifficultyConfig::medio(),
    ] {
        let pos = Position::from_fen(fen).unwrap();
        let result = Search::new().best_move(&pos, &config);
        assert!(matches!(result, SearchResult::EngineMove(_, _)));
    }
}
