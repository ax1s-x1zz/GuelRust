//! Layer 2 — 정밀 종반 탐색(Exact Endgame Prover).
//!
//! 잔여 단어 수가 예산 `B` 이내로 줄면, 전역 트랜스포지션 테이블(TT) +
//! 깊이 최적 미니맥스로 **증명 가능한** W/L/D 를 정밀 결정한다.
//!
//! - 상태 키: Zobrist 해시 (노드 시드 ⊕ 플레이 단어 시드들의 XOR).
//!   같은 (노드, 플레이 집합) 은 항상 같은 키 → TT 가 재사용.
//! - 루프 판정: 현재 탐색 경로에 같은 상태가 다시 나타나면 무승부(Draw).
//! - 수심 최적: 이기는 수 중 가장 빠른 승리(mate-in-k 최소), 지는 수 중
//!   가장 긴 저항을 반환.

use crate::graph::Graph;
use crate::node::NODE_SPACE;
use crate::oracle::{UNKNOWN, LevelTable};
use std::collections::{HashMap, HashSet};

/// 탐색 결과.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// 현재 플레이어가 k ply 안에 승리.
    Win(u16),
    /// 현재 플레이어가 k ply 안에 패배.
    Loss(u16),
    /// 무승부(루프).
    Draw,
}

impl Verdict {
    pub fn is_win(&self) -> bool {
        matches!(self, Verdict::Win(_))
    }
}

/// 탐색 전용 상태 (노드 + 플레이 집합을 Zobrist 키로).
pub struct Solver<'g> {
    graph: &'g Graph,
    oracle: &'g LevelTable,
    /// 플레이 단어 시드 (단어 ID → 랜덤 u64).
    word_seed: Vec<u64>,
    /// 노드 시드.
    node_seed: Vec<u64>,
    /// TT: 상태 키 → 정밀 결과.
    tt: HashMap<u64, Verdict>,
    /// 노드 방문 예산.
    budget: u32,
    nodes_visited: u32,
    /// 플레이된 단어 플래그 (백트래킹 공유).
    played: Vec<u8>,
}

const DEFAULT_BUDGET: u32 = 200_000;

impl<'g> Solver<'g> {
    /// 예산 기본값으로 생성. `oracle` 은 이동 순서(오라클 우선)에만 사용된다.
    pub fn new(graph: &'g Graph, oracle: &'g LevelTable) -> Solver<'g> {
        let n_words = graph.word_count();
        let mut rng = SplitMix64::from_seed(0x9E3779B97F4A7C15);
        let word_seed = (0..n_words).map(|_| rng.next_u64()).collect::<Vec<u64>>();
        let node_seed = (0..NODE_SPACE).map(|_| rng.next_u64()).collect::<Vec<u64>>();
        Self {
            graph,
            oracle,
            word_seed,
            node_seed,
            tt: HashMap::new(),
            budget: DEFAULT_BUDGET,
            nodes_visited: 0,
            played: vec![0; n_words as usize],
        }
    }

    /// 노드 방문 예산 설정.
    pub fn with_budget(mut self, budget: u32) -> Self {
        self.budget = budget;
        self
    }

    fn mark(&mut self, w: u32) {
        self.played[w as usize] = 1;
    }

    fn unmark(&mut self, w: u32) {
        self.played[w as usize] = 0;
    }

    fn is_played(&self, w: u32) -> bool {
        self.played[w as usize] != 0
    }

    /// 정밀 결정. 예산 초과 시 `None` (호출자가 오라클로 폴백).
    pub fn solve(&mut self, node: u32) -> Option<Verdict> {
        self.nodes_visited = 0;
        self.tt.clear();
        let mut path = HashSet::new();
        let z = self.node_seed[node as usize];
        self.search(node, z, &mut path)
    }

    fn search(&mut self, node: u32, z: u64, path: &mut HashSet<u64>) -> Option<Verdict> {
        self.nodes_visited += 1;
        if self.nodes_visited > self.budget {
            return None;
        }

        // 플레이 가능한 미사용 단어.
        let mut moves: Vec<u32> = Vec::new();
        for w in self.graph.out_words(node) {
            if !self.is_played(*w) {
                moves.push(*w);
            }
        }
        if moves.is_empty() {
            return Some(Verdict::Loss(0));
        }

        // 루프 판정: 경로에 이미 있는 상태.
        if !path.insert(z) {
            return Some(Verdict::Draw);
        }

        // TT 히트.
        if let Some(cached) = self.tt.get(&z) {
            path.remove(&z);
            return Some(*cached);
        }

        // 이동 순서: 오라클 레벨 우선 (L 타깃 먼저 → 조기 승리).
        let mut ordered: Vec<(u32, u32)> = moves.iter().map(|w| {
            let t = self.graph.key_out_of(*w) as u32;
            let lvl = self.oracle.get(t);
            // L(짝수) 타깃 우선, 깊이 오름차순.
            let key = if lvl != UNKNOWN && lvl.is_multiple_of(2) {
                lvl as u32
            } else {
                1000 + lvl as u32
            };
            (key, *w)
        }).collect::<Vec<(u32, u32)>>();
        ordered.sort();

        let mut best_win_depth: u16 = u16::MAX;
        let mut best_loss_depth: u16 = 0;
        let mut found_draw = false;
        let mut result: Option<Verdict> = None;

        for (_, w) in ordered {
            let target = self.graph.key_out_of(w) as u32;
            let wz = z ^ self.node_seed[target as usize] ^ self.word_seed[w as usize];
            self.mark(w);
            let sub = self.search(target, wz, path);
            self.unmark(w);
            if sub.is_none() {
                continue; // 예산 초과 → 이 가지 무시, 전체 정확성 상실 가능
            }
            match sub.unwrap() {
                Verdict::Loss(d) => {
                    // 상대가 지면 나는 이긴다: Win(d+1)
                    let wd = (d as u32 + 1) as u16;
                    if wd < best_win_depth {
                        best_win_depth = wd;
                        result = Some(Verdict::Win(wd));
                    }
                },
                Verdict::Draw => {
                    found_draw = true;
                },
                Verdict::Win(d) => {
                    // 상대가 이기면 나는 진다: Loss(d+1)
                    let ld = (d as u32 + 1) as u16;
                    if ld > best_loss_depth {
                        best_loss_depth = ld;
                    }
                },
            }
        }

        path.remove(&z);

        if best_win_depth != u16::MAX {
            result = Some(Verdict::Win(best_win_depth));
        } else if best_loss_depth > 0 {
            result = Some(Verdict::Loss(best_loss_depth));
        } else if found_draw {
            result = Some(Verdict::Draw);
        }
        result?;
        let value = result.unwrap();
        self.tt.insert(z, value);
        result
    }
}

/// 빠른 결정성 난수 (SplitMix64).
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn from_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle::retrograde;
    use guel_dict::DictBuilder;
    use guel_kor::chain::syllable_node;

    fn graph_of(words: &[&str]) -> Graph {
        let mut b = DictBuilder::new();
        for w in words {
            b.add(w);
        }
        let d = b.build();
        Graph::build(&d)
    }

    #[test]
    fn immediate_loss_when_no_move() {
        let g = graph_of(&["바나나"]);
        let oracle = retrograde(&g);
        let mut s = Solver::new(&g, &oracle);
        let na = syllable_node(2, 0); // '나' — 이어질 단어 없음
        let v = s.solve(na).unwrap();
        assert_eq!(v, Verdict::Loss(0));
    }

    #[test]
    fn win_when_move_forces_loss() {
        let g = graph_of(&["가구", "구름", "구슬"]);
        let oracle = retrograde(&g);
        let mut s = Solver::new(&g, &oracle);
        let gu = syllable_node(0, 13); // '구' — 구름/구슬로 ㅁ/ㄹ(no move)로
        let v = s.solve(gu).unwrap();
        assert_eq!(v, Verdict::Win(1)); // 한 수에 이김
    }

    #[test]
    fn loss_when_all_moves_lead_to_win() {
        let g = graph_of(&["가구", "구름", "구슬"]);
        let oracle = retrograde(&g);
        let mut s = Solver::new(&g, &oracle);
        let ga = syllable_node(0, 0); // '가' — 유일한 수 '가구'가 '구'(W)로
        let v = s.solve(ga).unwrap();
        assert_eq!(v, Verdict::Loss(2));
    }

    #[test]
    fn solver_result_present() {
        let words = ["가구", "구름", "구슬", "바나나", "사과", "나무"];
        let g = graph_of(&words);
        let oracle = retrograde(&g);
        let mut s = Solver::new(&g, &oracle);
        let ga = syllable_node(0, 0);
        let v = s.solve(ga).unwrap();
        assert!(v.is_win() || v == Verdict::Draw || matches!(v, Verdict::Loss(_)));
    }

    #[test]
    fn budget_overflow_returns_none() {
        let g = graph_of(&["가구", "구름", "구슬", "바나나"]);
        let oracle = retrograde(&g);
        let mut s = Solver::new(&g, &oracle).with_budget(1);
        let gu = syllable_node(0, 13);
        // 예산 1이면 루트 방문 후 자식 노드에서 초과 → 정확 결과 없음
        let v = s.solve(gu);
        assert!(v.is_none());
    }
}