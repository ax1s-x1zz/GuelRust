//! `guel-core` — 구엘룰 엔진 코어.
//!
//! 그래프 구축, 정적 오라클(Layer1) 역방향 전파, 정밀 종반 탐색(Layer2),
//! 증분 갱신, 스왑 상태를 구현한다.

pub mod engine;
pub mod game;
pub mod graph;
pub mod node;
pub mod oracle;
pub mod solver;

pub use engine::{BUDGET, Engine, Move, Side};
pub use game::{Played, ResidualStats};
pub use graph::Graph;
pub use node::{NODE_SPACE, is_consonant, is_syllable};
pub use oracle::{UNKNOWN, LevelQuery, LevelTable, is_loss, is_neutral, is_win, retrograde};
pub use solver::{SplitMix64, Verdict};