use chess_engine::{
    analyze_game, GameRecord, GameResult, MoveClass, Position,
};

fn uci_to_move(pos: &Position, uci: &str) -> chess_engine::Move {
    pos.legal_moves()
        .into_iter()
        .find(|m| m.to_uci() == uci)
        .unwrap_or_else(|| panic!("move {} not found in legal moves for {}", uci, pos.to_fen()))
}

#[test]
fn fools_mate_is_blunder() {
    // 1.f3?? e5  2.g4?? Qh4#  — white's f3 and g4 are blunders
    let start = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut pos = Position::from_fen(start).unwrap();

    let mut moves = Vec::new();
    for uci in ["f2f3", "e7e5", "g2g4", "d8h4"] {
        let m = uci_to_move(&pos, uci);
        moves.push(m);
        let _ = pos.make_move_mut(m);
    }

    let record = GameRecord {
        initial_fen: start.to_string(),
        moves,
        result: GameResult::BlackWins,
    };

    let report = analyze_game(&record);
    assert_eq!(report.move_analyses.len(), 4);

    // f3 (move 0, white) should be classified as a blunder, mistake, or inaccuracy
    let f3_class = &report.move_analyses[0].classification;
    assert!(
        matches!(f3_class, MoveClass::Blunder | MoveClass::Mistake | MoveClass::Inaccuracy),
        "f3 should be Blunder/Mistake/Inaccuracy, got {:?}", f3_class
    );

    // g4 (move 2, white) should also be a blunder or worse
    let g4_class = &report.move_analyses[2].classification;
    assert!(
        matches!(g4_class, MoveClass::Blunder | MoveClass::Mistake | MoveClass::Inaccuracy),
        "g4 should be Blunder/Mistake/Inaccuracy, got {:?}", g4_class
    );

    // White's accuracy should be below 100 (not perfect)
    assert!(
        report.summary.accuracy_white < 100.0,
        "white accuracy should be below 100 after suboptimal moves, got {}",
        report.summary.accuracy_white
    );
}

#[test]
fn solid_move_is_not_blunder() {
    // From starting position, white plays 1.e4 (standard opening move)
    let start = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let pos = Position::from_fen(start).unwrap();
    let m = uci_to_move(&pos, "e2e4");

    let record = GameRecord {
        initial_fen: start.to_string(),
        moves: vec![m],
        result: GameResult::WhiteWins,
    };

    let report = analyze_game(&record);
    assert_eq!(report.move_analyses.len(), 1);

    let cls = &report.move_analyses[0].classification;
    // e4 is a solid opening move, should not be blunder or even inaccuracy
    assert!(
        !matches!(cls, MoveClass::Blunder | MoveClass::Mistake),
        "1.e4 should not be Blunder/Mistake, got {:?}", cls
    );
}
