use chess_engine::Position;

fn perft(fen: &str, depth: u8) -> u64 {
    Position::from_fen(fen).unwrap().perft(depth)
}

// ── Startpos ──────────────────────────────────────────────────────────────────

#[test]
fn startpos_d1() { assert_eq!(perft("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 1), 20); }

#[test]
fn startpos_d2() { assert_eq!(perft("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 2), 400); }

#[test]
fn startpos_d3() { assert_eq!(perft("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 3), 8_902); }

#[test]
fn startpos_d4() { assert_eq!(perft("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 4), 197_281); }

#[test]
#[ignore]
fn startpos_d5() { assert_eq!(perft("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 5), 4_865_609); }

// ── Kiwipete ──────────────────────────────────────────────────────────────────

#[test]
fn kiwipete_d1() {
    assert_eq!(perft("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 1), 48);
}

#[test]
fn kiwipete_d2() {
    assert_eq!(perft("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 2), 2_039);
}

#[test]
fn kiwipete_d3() {
    assert_eq!(perft("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 3), 97_862);
}

#[test]
#[ignore]
fn kiwipete_d4() {
    assert_eq!(perft("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 4), 4_085_603);
}

// ── Position 3 ────────────────────────────────────────────────────────────────

#[test]
fn pos3_d1() { assert_eq!(perft("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - -", 1), 14); }

#[test]
fn pos3_d2() { assert_eq!(perft("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - -", 2), 191); }

#[test]
fn pos3_d3() { assert_eq!(perft("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - -", 3), 2_812); }

#[test]
#[ignore]
fn pos3_d5() { assert_eq!(perft("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - -", 5), 674_624); }

// ── Position 5 ────────────────────────────────────────────────────────────────

#[test]
fn pos5_d1() { assert_eq!(perft("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", 1), 44); }

#[test]
fn pos5_d2() { assert_eq!(perft("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", 2), 1_486); }

#[test]
#[ignore]
fn pos5_d4() { assert_eq!(perft("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", 4), 2_103_487); }
