//! 엔진 — 수심 최적 정책 + 사이드 선택 + Layer1/2 결합.
//!
//! - 잔여 오라클(Layer1)로 모든 노드의 현재 레벨 유지 (증분 재계산).
//! - 잔여 단어 수 ≤ `B` 가 되면 Layer2 정밀 탐색으로 수심을 확정.
//! - 수심 최적: W면 최단 강제 승리(mate-in-k), L면 최장 저항, D면 중립 정책.

use crate::game::{Played, ResidualStats};
use crate::graph::Graph;
use crate::oracle::{is_loss, is_neutral, is_win, retrograde, LevelTable};
use crate::solver::{Solver, Verdict};

/// 수심 정밀 탐색 예산 (잔여 단어 수).
pub const BUDGET: u32 = 30;

/// 필승 사이드.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// 선공 (먼저 두기).
    First,
    /// 후공 (상대가 먼저).
    Second,
    /// 어느 쪽이든 필승.
    Either,
}

/// 최선수 결과.
#[derive(Debug, Clone, Copy)]
pub struct Move {
    pub word_id: u32,
    /// 해당 수의 타깃 노드.
    pub target: u32,
    /// 타깃 노드의 현재(잔여) 레벨.
    pub target_level: u8,
}

/// 구엘룰 엔진.
pub struct Engine<'g> {
    graph: &'g Graph,
    oracle: &'g LevelTable,
    played: Played,
    residual: LevelTable,
    unplayed_total: u32,
}

impl<'g> Engine<'g> {
    pub fn new(graph: &'g Graph, oracle: &'g LevelTable) -> Engine<'g> {
        let n = graph.word_count();
        Self {
            graph,
            oracle,
            played: Played::new(n),
            residual: retrograde(graph),
            unplayed_total: n,
        }
    }

    /// 단어를 플레이하고 잔여 레벨을 재계산한다.
    pub fn play(&mut self, word_id: u32) {
        self.played.play(word_id);
        self.unplayed_total -= 1;
        self.rebuild_residual();
    }

    /// 플레이를 되돌린다 (백트래킹).
    pub fn unplay(&mut self, word_id: u32) {
        self.played.unplay(word_id);
        self.unplayed_total += 1;
        self.rebuild_residual();
    }

    /// 노드의 현재(잔여) 레벨.
    pub fn classify(&self, node: u32) -> u8 {
        self.residual.get(node)
    }

    /// 노드 분류 문자열.
    pub fn classify_str(&self, node: u32) -> &str {
        self.residual.classify(node)
    }

    /// 초기(미플레이) 오라클 레벨 테이블.
    pub fn full_oracle(&self) -> &LevelTable {
        self.oracle
    }

    /// 현재(잔여) 레벨 테이블.
    pub fn residual_table(&self) -> &LevelTable {
        &self.residual
    }

    /// 단어 ID → 끝소리 노드.
    pub fn current_key_out(&self, word_id: u32) -> u32 {
        self.graph.key_out_of(word_id) as u32
    }

    /// 시작 노드의 필승 사이드 판정.
    pub fn start_side(&self, node: u32) -> Side {
        let lvl = self.residual.get(node);
        if is_win(lvl) {
            Side::First
        } else if is_loss(lvl) {
            Side::Second
        } else {
            Side::Either
        }
    }

    /// 플레이 가능한 미사용 단어 ID.
    pub fn playable(&self, node: u32) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        for w in self.graph.out_words(node).iter() {
            if !self.played.is_played(*w) {
                out.push(*w);
            }
        }
        out
    }

    /// 수심 최적 최선수. 둘 곳이 없으면 `None`.
    pub fn best_move(&mut self, node: u32) -> Option<Move> {
        let moves = self.playable(node);
        if moves.is_empty() {
            return None;
        }

        // Layer2 정밀 확정: 잔여 단어 수가 예산 이내.
        if self.unplayed_total <= BUDGET {
            let mut s = Solver::new(self.graph, &self.residual);
            if let Some(v) = s.solve(node) {
                return self.pick_by_verdict(moves, v);
            }
        }

        self.pick_by_oracle(node, moves)
    }

    fn rebuild_residual(&mut self) {
        let stats = ResidualStats::build(self.graph, &self.played);
        self.residual = retrograde(&stats);
    }

    /// Layer2 판정에 따른 최선수 (수심 최적).
    fn pick_by_verdict(&self, moves: Vec<u32>, v: Verdict) -> Option<Move> {
        let best = match v {
            // 이김: 상대가 가장 짧게 지는 수 (Loss(d-1) 목표).
            Verdict::Win(_) => {
                let mut best: Option<(u16, u32)> = None;
                for w in moves {
                    let t = self.graph.key_out_of(w) as u32;
                    let sub = self.residual.get(t);
                    if is_loss(sub) {
                        let key = sub as u16; // 깊이 오름차순 최소
                        if best.is_none() || key < best.unwrap().0 {
                            best = Some((key, w));
                        }
                    }
                }
                best
            },
            // 짐: 최장 저항 (상대 W 깊이 최대).
            Verdict::Loss(_) => {
                let mut best: Option<(u16, u32)> = None;
                for w in moves {
                    let t = self.graph.key_out_of(w) as u32;
                    let sub = self.residual.get(t);
                    if is_win(sub) {
                        let key = u16::MAX - sub as u16; // 깊이 내림차순 최대
                        if best.is_none() || key > best.unwrap().0 {
                            best = Some((key, w));
                        }
                    }
                }
                best
            },
            // 무승부: D 유지 우선, 없으면 최장 연장.
            Verdict::Draw => {
                let mut best: Option<(u16, u32)> = None;
                for w in moves {
                    let t = self.graph.key_out_of(w) as u32;
                    let sub = self.residual.get(t);
                    let key = if is_loss(sub) {
                        u16::MAX // L 목표 우선
                    } else if is_neutral(sub) {
                        u16::MAX - 1
                    } else {
                        u16::MAX - sub as u16
                    };
                    if best.is_none() || key > best.unwrap().0 {
                        best = Some((key, w));
                    }
                }
                best
            },
        };
        best.map(|(_, w)| Move {
            word_id: w,
            target: self.graph.key_out_of(w) as u32,
            target_level: self.residual.get(self.graph.key_out_of(w) as u32),
        })
    }

    /// 오라클 정책에 따른 최선수.
    fn pick_by_oracle(&self, node: u32, moves: Vec<u32>) -> Option<Move> {
        let cur = self.residual.get(node);
        let mut best: Option<(u16, u32)> = None;
        for w in moves {
            let t = self.graph.key_out_of(w) as u32;
            let sub = self.residual.get(t);
            let key = move_key(cur, sub);
            if best.is_none() || key > best.unwrap().0 {
                best = Some((key, w));
            }
        }
        best.map(|(_, w)| Move {
            word_id: w,
            target: self.graph.key_out_of(w) as u32,
            target_level: self.residual.get(self.graph.key_out_of(w) as u32),
        })
    }
}

/// 수심 최적 이동 키 (큰 값 우선).
///
/// - 현재 W: 상대를 L(최소 깊이)로 → `2^k - depth`.
/// - 현재 L: 상대를 W(최대 깊이)로 → 깊이 그대로 (최장 저항).
/// - 현재 D: L 목표 > D 유지 > W(최장 연장).
fn move_key(cur: u8, tgt: u8) -> u16 {
    let t = tgt as u16;
    if is_win(cur) {
        // 이기려면 L 목표 최소 깊이. 우선 L.
        if is_loss(tgt) {
            (1u16 << 12) - t
        } else {
            0
        }
    } else if is_loss(cur) {
        // 지는 위치: W 목표 최대 깊이 (최장 저항).
        if is_win(tgt) {
            t
        } else {
            0
        }
    } else {
        // 중립: L > D > W(연장).
        if is_loss(tgt) {
            (1u16 << 13) + t
        } else if is_neutral(tgt) {
            1u16 << 12
        } else if is_win(tgt) {
            t
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NODE_SPACE;
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
    fn side_selection() {
        let g = graph_of(&["가구", "구름", "구슬"]);
        let o = retrograde(&g);
        let e = Engine::new(&g, &o);
        let ga = syllable_node(0, 0); // '가' L2
        let gu = syllable_node(0, 13); // '구' W1
        assert_eq!(e.start_side(gu), Side::First);
        assert_eq!(e.start_side(ga), Side::Second);
    }

    #[test]
    fn best_move_shortest_win() {
        let g = graph_of(&["가구", "구름", "구슬"]);
        let o = retrograde(&g);
        let mut e = Engine::new(&g, &o);
        let gu = syllable_node(0, 13); // '구' W1
        let m = e.best_move(gu).unwrap();
        // '구'의 수는 ㅁ/ㄹ(L0)로 보내는 구름/구슬 → target L0
        assert!(is_loss(m.target_level));
    }

    #[test]
    fn play_consumes_word() {
        let g = graph_of(&["가구", "구름", "구슬"]);
        let o = retrograde(&g);
        let mut e = Engine::new(&g, &o);
        let gu = syllable_node(0, 13);
        let m = e.best_move(gu).unwrap();
        e.play(m.word_id);
        // '구'에서 남은 수가 하나 줄었어야 함
        let remaining = e.playable(gu).len();
        assert_eq!(remaining, 1);
        e.unplay(m.word_id);
        assert_eq!(e.playable(gu).len(), 2);
    }

    #[test]
    fn no_move_returns_none() {
        let g = graph_of(&["바나나"]);
        let o = retrograde(&g);
        let mut e = Engine::new(&g, &o);
        let na = syllable_node(2, 0);
        assert!(e.best_move(na).is_none());
    }

    #[test]
    fn node_space_bounds_sanity() {
        const { assert!(NODE_SPACE > 0) };
    }
}