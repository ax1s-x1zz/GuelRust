//! 자가검증(rollout) — 필승 포지션에서 엔진이 절대 지지 않는지 시뮬레이션.
//!
//! FR-Q1: W 시작 노드에서 엔진(수심 최적 정책) vs 무작위/그리디 상대 대결을
//! 여러 시드로 반복해 패배 0회를 검증한다.

use crate::engine::{Engine, Move};
use crate::graph::Graph;
use crate::oracle::{is_win, retrograde};
use crate::solver::SplitMix64;

/// 대결 결과.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// 엔진 승리.
    EngineWin,
    /// 엔진 패배 (필승 위치에서 발생하면 버그).
    EngineLoss,
    /// 무승부(루프) — 단어 한도 초과.
    Draw,
}

const MAX_PLIES: u32 = 2000;

/// 무작위 상대 전략 시드로 엔진과 대결한다.
pub fn simulate_engine_vs_random(
    graph: &Graph,
    start_node: u32,
    seed: u64,
) -> Outcome {
    let oracle = retrograde(graph);
    if !is_win(oracle.get(start_node)) {
        // W 포지션이 아니면 검증 대상이 아님.
        return Outcome::Draw;
    }
    let mut engine = Engine::new(graph.clone(), oracle);
    let mut rng = SplitMix64::from_seed(seed);
    let mut node = start_node;
    let mut engine_turn = true;
    let mut plies = 0;

    loop {
        plies += 1;
        if plies > MAX_PLIES {
            return Outcome::Draw;
        }
        let mv: Option<Move> = if engine_turn {
            engine.best_move(node)
        } else {
            // 상대: 무작위 유효수.
            let legal = engine.playable(node);
            if legal.is_empty() {
                return Outcome::EngineWin; // 상대 패배 = 엔진 승
            }
            let pick = rng.next_u64() as usize % legal.len();
            Some(Move {
                word_id: legal[pick],
                target: 0,
                target_level: 0,
            })
        };
        // 필승 위치에서는 반드시 수가 있어야 함.
        if mv.is_none() {
            return Outcome::EngineLoss;
        }
        let m = mv.unwrap();
        node = graph.key_out_of(m.word_id) as u32;
        engine.play(m.word_id);
        engine_turn = !engine_turn;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guel_dict::DictBuilder;
    use guel_kor::chain::syllable_node;

    /// 검증용 합성 사전 — 막다른 음절(리/무 등) 체인을 포함해 W 노드를 생성.
    fn build_dict() -> Graph {
        let words = [
            "가구", "구름", "구슬", "구두", "두부", "부엌",
            "파리", "리본", "본보기", "기본", "기와", "와이파이",
            "해파리", "소나무", "무지개", "개구리", "가방", "방문",
            "문구", "구경", "경찰",
        ];
        let mut b = DictBuilder::new();
        for w in words {
            b.add(w);
        }
        let d = b.build();
        Graph::build(&d)
    }

    #[test]
    fn engine_never_loses_from_win_start() {
        let g = build_dict();
        let oracle = retrograde(&g);
        let mut win_nodes = 0;
        let mut losses = 0;
        let mut draws = 0;
        for seed in 0..20u64 {
            for node in 0..crate::NODE_SPACE {
                if !is_win(oracle.get(node)) {
                    continue;
                }
                win_nodes += 1;
                let out = simulate_engine_vs_random(&g, node, seed);
                match out {
                    Outcome::EngineWin => {},
                    Outcome::EngineLoss => losses += 1,
                    Outcome::Draw => draws += 1,
                }
            }
        }
        assert!(win_nodes > 0);
        assert_eq!(losses, 0, "필승 위치에서 엔진이 패배하면 안 됨");
        // 루프(Draw)는 무작위 상대가 루프에 빠질 때만 발생 — 패배가 아니면 허용.
        let _ = draws;
    }

    #[test]
    fn engine_wins_specific_win_node() {
        let g = build_dict();
        // '구' 노드가 W 인지 확인 후 대결
        let gu = syllable_node(0, 13);
        let oracle = retrograde(&g);
        if is_win(oracle.get(gu)) {
            for seed in 0..10u64 {
                assert_eq!(
                    simulate_engine_vs_random(&g, gu, seed),
                    Outcome::EngineWin,
                );
            }
        }
    }
}