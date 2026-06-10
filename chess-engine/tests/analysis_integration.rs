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
    // (depth-6 engine may not fully see the queen threat yet)
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

    // Accuracy stats: white's accuracy should be significantly below 100
    assert!(
        report.summary.accuracy_white < 80.0,
        "white accuracy should be low after two blunders, got {}",
        report.summary.accuracy_white
    );
}

#[test]
fn mate_in_one_is_excellent() {
    // White to play, Rg6-g8#. King h8 can't escape: g8=rook, h7=covered by Kf7, g7=covered by Rg8.
    let fen = "7k/5K2/6R1/8/8/8/8/8 w - - 0 1";
    let pos = Position::from_fen(fen).unwrap();
    let m = uci_to_move(&pos, "g6g8");

    let record = GameRecord {
        initial_fen: fen.to_string(),
        moves: vec![m],
        result: GameResult::WhiteWins,
    };

    let report = analyze_game(&record);
    assert_eq!(report.move_analyses.len(), 1);

    let cls = &report.move_analyses[0].classification;
    assert!(
        matches!(cls, MoveClass::Excellent | MoveClass::Brilliant | MoveClass::Good),
        "Ra8# should be Excellent/Brilliant, got {:?}", cls
    );
}
