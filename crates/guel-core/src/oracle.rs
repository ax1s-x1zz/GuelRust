//! 정적 오라클(Layer 1) — 전 그래프 역방향 완전 분류.
//!
//! PRD §4.2 점화 규칙을 따르는 레벨 고정점 전파로 모든 노드에
//! W(n)/L(n)/D(중립) 를 할당한다.
//!
//! - `L0`: 방어 단어가 0개인 노드 (즉시 패배).
//! - `W(n+1)`: L(≤n) 으로 보내는 수가 존재하는 노드.
//! - `L(n+1)`: 모든 수가 W(≤n) 으로 가는 노드.
//! - 미분류 잔여 = 중립(D).

use crate::graph::Graph;
use crate::node::NODE_SPACE;
use std::collections::VecDeque;

/// 레벨 값 인코딩:
/// - `UNKNOWN = 255` (중립 D 포함)
/// - `0` = L0, 짝수 = L(k), 홀수 = W(k)
pub const UNKNOWN: u8 = 255;

/// L0 를 뜻하는 레벨 값.
pub const L0: u8 = 0;

/// 레벨 값이 W(선공 유리)인지.
#[inline]
pub fn is_win(level: u8) -> bool {
    level != UNKNOWN && level % 2 == 1
}

/// 레벨 값이 L(선공 패배)인지.
#[inline]
pub fn is_loss(level: u8) -> bool {
    level != UNKNOWN && level.is_multiple_of(2)
}

/// 레벨 값이 중립(D)인지.
#[inline]
pub fn is_neutral(level: u8) -> bool {
    level == UNKNOWN
}

/// 레벨이 의미하는 수심(ply): L0=0, W1=1, L1=2, ...
#[inline]
pub fn depth_of(level: u8) -> u8 {
    level
}

/// 노드별 레벨 테이블.
pub struct LevelTable {
    levels: Vec<u8>,
}

impl LevelTable {
    pub fn get(&self, node: u32) -> u8 {
        self.levels[node as usize]
    }

    /// 노드 분류 문자열 (`L0`, `W3`, `D` 등).
    pub fn classify(&self, node: u32) -> &str {
        let l = self.get(node);
        match l {
            UNKNOWN => "D",
            _ if is_win(l) => "W",
            _ => "L",
        }
    }

    /// 모든 노드 레벨 (테스트·덤프용).
    pub fn raw(&self) -> &[u8] {
        &self.levels
    }
}

/// 역방향 전파 입력 — 노드별 서로 다른 이웃 수와 역방향 소스 목록.
///
/// 전체 그래프(`Graph`)와 잔여 그래프(플레이된 단어 제외)가 공통으로
/// 구현하는 최소 인터페이스다.
pub trait LevelQuery {
    /// 노드의 서로 다른 이웃(플레이 가능한 타깃) 수.
    fn out_deg(&self, node: u32) -> u32;
    /// 노드로 도달하는 서로 다른 소스 노드 (정렬).
    fn predecessors(&self, node: u32) -> &[u16];
}

impl LevelQuery for Graph {
    fn out_deg(&self, node: u32) -> u32 {
        self.out_deg(node)
    }

    fn predecessors(&self, node: u32) -> &[u16] {
        self.predecessors(node)
    }
}

/// 역방향 전파로 레벨 테이블을 산출한다.
pub fn retrograde<Q: LevelQuery>(query: &Q) -> LevelTable {
    let n = NODE_SPACE as usize;
    let mut levels = vec![UNKNOWN; n];
    let mut cnt_win: Vec<u32> = vec![0; n];
    // (노드, 레벨) BFS 큐
    let mut queue: VecDeque<(u32, u8)> = VecDeque::new();

    // L0: 방어 단어 0개 노드
    for node in 0..NODE_SPACE {
        if query.out_deg(node) == 0 {
            levels[node as usize] = L0;
            queue.push_back((node, L0));
        }
    }

    while let Some((v, lvl)) = queue.pop_front() {
        if is_win(lvl) {
            // v 가 W → v 로 도달하는 모든 소스 u 의 W-이웃 수 증가.
            for u in query.predecessors(v) {
                let ui = *u as usize;
                if levels[ui] == UNKNOWN {
                    cnt_win[ui] += 1;
                    if cnt_win[ui] == query.out_deg(*u as u32) {
                        // 모든 이웃이 W → u 는 L(lvl+1)
                        levels[ui] = lvl + 1;
                        queue.push_back((*u as u32, lvl + 1));
                    }
                }
            }
        } else {
            // v 가 L → v 로 도달하는 미분류 소스 u 는 즉시 W(lvl+1)
            for u in query.predecessors(v) {
                let ui = *u as usize;
                if levels[ui] == UNKNOWN {
                    levels[ui] = lvl + 1;
                    queue.push_back((*u as u32, lvl + 1));
                }
            }
        }
    }

    LevelTable { levels }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guel_dict::DictBuilder;
    use guel_kor::chain::syllable_node;

    fn table_of(words: &[&str]) -> LevelTable {
        let mut b = DictBuilder::new();
        for w in words {
            b.add(w);
        }
        let d = b.build();
        let g = Graph::build(&d);
        retrograde(&g)
    }

    #[test]
    fn dead_end_is_l0() {
        // 바나나 -> '나'(ㄴ+ㅏ) 로 이어질 단어가 없음 -> L0
        let t = table_of(&["바나나"]);
        let na = syllable_node(2, 0);
        assert_eq!(t.get(na), L0);
        assert!(is_loss(t.get(na)));
    }

    #[test]
    fn win_when_move_to_l0() {
        // 가구(가..구), 구름(구..름), 구슬(구..슬)
        // ㅁ/ㄹ 노드(dead-end) = L0, '구'는 ㅁ/ㄹ로 보냄 -> W1
        let t = table_of(&["가구", "구름", "구슬"]);
        let gu = syllable_node(0, 13); // '구'
        assert!(is_win(t.get(gu)));
        assert_eq!(depth_of(t.get(gu)), 1);
        // ㅁ(6), ㄹ(5) 자음 노드 = L0
        assert_eq!(t.get(6), L0);
        assert_eq!(t.get(5), L0);
    }

    #[test]
    fn loss_when_all_moves_to_win() {
        // '가'는 유일한 수 '가구'가 '구'(W1) 로 감 -> L2
        let t = table_of(&["가구", "구름", "구슬"]);
        let ga = syllable_node(0, 0); // '가'
        assert!(is_loss(t.get(ga)));
        assert_eq!(depth_of(t.get(ga)), 2);
    }

    #[test]
    fn oracle_self_consistency() {
        // 모든 노드에 대해:
        // - W 이면 L(≤level) 로 가는 수 존재
        // - L 이면 모든 수가 W(<level) 로 감
        let mut b = DictBuilder::new();
        for w in ["가구", "구름", "구슬", "바나나", "사과"] {
            b.add(w);
        }
        let d = b.build();
        let g = Graph::build(&d);
        let t = retrograde(&g);
        for node in 0..NODE_SPACE {
            let l = t.get(node);
            if is_win(l) {
                let has_move_to_loss = g
                    .out_neighbors(node)
                    .iter()
                    .any(|v| is_loss(t.get(*v as u32)));
                assert!(has_move_to_loss, "W 노드는 L로 가는 수가 있어야 함");
            } else if is_loss(l) {
                let all_moves_to_win = g
                    .out_neighbors(node)
                    .iter()
                    .all(|v| is_win(t.get(*v as u32)));
                assert!(all_moves_to_win, "L 노드의 모든 수는 W로 가야 함");
            }
        }
    }

    #[test]
    fn neutral_when_loop() {
        // 가(가), 가가(가..가) 와 같이 무한 루프 가능:
        // '가'에서 '가'로 루프 -> 중립(D) 또는 W
        let t = table_of(&["가", "가가"]);
        let ga = syllable_node(0, 0);
        let l = t.get(ga);
        assert!(l == UNKNOWN || is_win(l));
    }
}
