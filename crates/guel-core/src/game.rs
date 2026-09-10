//! 게임 상태 — 플레이된 단어 추적과 잔여 그래프 쿼리.

use crate::graph::Graph;
use crate::node::NODE_SPACE;
use crate::oracle::LevelQuery;
use std::collections::HashSet;

/// 플레이된 단어 추적 (재귀 백트래킹 지원).
pub struct Played {
    flags: Vec<u8>,
    count: u32,
}

impl Played {
    pub fn new(word_count: u32) -> Self {
        Self {
            flags: vec![0; word_count as usize],
            count: 0,
        }
    }

    pub fn play(&mut self, word_id: u32) {
        debug_assert!(self.flags[word_id as usize] == 0);
        self.flags[word_id as usize] = 1;
        self.count += 1;
    }

    pub fn unplay(&mut self, word_id: u32) {
        debug_assert!(self.flags[word_id as usize] == 1);
        self.flags[word_id as usize] = 0;
        self.count -= 1;
    }

    pub fn is_played(&self, word_id: u32) -> bool {
        self.flags[word_id as usize] != 0
    }

    pub fn count(&self) -> u32 {
        self.count
    }
}

/// 잔여 그래프 통계 — 플레이되지 않은 단어로 재구성한 역방향 전파 입력.
pub struct ResidualStats {
    out_deg: Vec<u32>,
    rev: Vec<Vec<u16>>,
}

impl ResidualStats {
    /// 현재 플레이 상태를 반영해 잔여 통계를 재구성한다.
    ///
    /// 전체 단어를 한 번 순회(O(W))하므로 수십 만 단어에서도 수 ms 이내.
    pub fn build(graph: &Graph, played: &Played) -> Self {
        let n = NODE_SPACE as usize;
        let mut targets: Vec<HashSet<u16>> = (0..n).map(|_| HashSet::new()).collect::<Vec<_>>();
        let mut rev: Vec<HashSet<u16>> = (0..n).map(|_| HashSet::new()).collect::<Vec<_>>();

        for w in 0..graph.word_count() {
            if played.is_played(w) {
                continue;
            }
            let ko = graph.key_out_of(w) as usize;
            for src in graph.word_sources(w) {
                let si = *src as usize;
                targets[si].insert(ko as u16);
                rev[ko].insert(*src);
            }
        }

        let out_deg = targets.iter().map(|s| s.len() as u32).collect::<Vec<u32>>();
        let rev_f = rev
            .iter()
            .map(|s| {
                let mut v = s.iter().copied().collect::<Vec<u16>>();
                v.sort();
                v
            })
            .collect::<Vec<Vec<u16>>>();
        Self { out_deg, rev: rev_f }
    }
}

impl LevelQuery for ResidualStats {
    fn out_deg(&self, node: u32) -> u32 {
        self.out_deg[node as usize]
    }

    fn predecessors(&self, node: u32) -> &[u16] {
        &self.rev[node as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle::{is_win, retrograde};
    use guel_dict::DictBuilder;
    use guel_kor::chain::syllable_node;

    fn setup() -> (Graph, Played) {
        let mut b = DictBuilder::new();
        for w in ["가구", "구름", "구슬", "바나나"] {
            b.add(w);
        }
        let d = b.build();
        let count = d.len() as u32;
        let g = Graph::build(&d);
        (g, Played::new(count))
    }

    #[test]
    fn residual_excludes_played() {
        let (g, mut played) = setup();
        let full = retrograde(&g);
        let before = full.get(syllable_node(2, 0)); // '나'
        assert_eq!(before, 0); // L0 (바나나만 있을 때 '나'로 이어지는 단어 없음)

        played.play(1); // 바나나 (id 1)
        let res = ResidualStats::build(&g, &played);
        let after = retrograde(&res).get(syllable_node(2, 0));
        assert_eq!(after, 0); // 여전히 L0

        // '가' 레벨은 플레이 여부와 무관하게 보존되어야 함 (바나나가 '가'와 무관)
        let res2 = retrograde(&ResidualStats::build(&g, &played));
        assert!(is_win(res2.get(syllable_node(0, 13)))); // '구' W
    }
}