mod bitboard;
pub mod eval;
mod moves;
mod movegen;
mod position;
pub mod search;
mod tables;
mod types;
pub mod analysis;

pub use analysis::{
    GameRecord, GameReport, GameResult, GameSummary,
    MoveAnalysis, MoveClass,
    analyze_game, classify, compute_accuracy,
    ANALYSIS_DEPTH, ANALYSIS_NODES,
};
pub use bitboard::Bitboard;
pub use moves::{Move, MoveFlag, SavedState};
pub use position::Position;
pub use search::{DifficultyConfig, Search, SearchResult, MATE_SCORE};
pub use types::{CastlingRights, Color, PieceType, Square};
