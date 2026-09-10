//! 이음 그래프 — 단어를 노드(자음/음절) 사이의 모서리로 변환.
//!
//! 단어 `w` 는 소스 노드 집합 `sources(w)` 에서 타깃 노드 `key_out(w)` 로 향하는
//! 모서리다. `sources(w)` 는 두음법칙 허용 집합을 반영해
//! - 첫 음절이 받침 없음 → 해당 음절 노드 단일
//! - 첫 음절이 받침 있음 → `key_in_cho` 로 시작 가능한 모든 끝 대표음 자음 노드
//!
//! 로 확장된다 (PRD §4.3.1).

use crate::node::{is_syllable, NODE_SPACE};
use guel_dict::Dict;
use guel_kor::chain::WordKey;
use guel_kor::duseum::inv_allowed_inits;
use std::collections::HashSet;

/// 구축된 이음 그래프 (불변).
pub struct Graph {
    /// 노드 키 → 해당 노드에서 플레이 가능한 단어 ID (중복 제거, 정렬).
    out_words: Vec<Vec<u32>>,
    /// 노드 키 → 도달 가능한 타깃 노드 (중복 제거, 정렬).
    out_neighbors: Vec<Vec<u16>>,
    /// 타깃 노드 → 소스 노드 (역방향, 중복 제거, 정렬).
    rev: Vec<Vec<u16>>,
    /// 노드 키 → 서로 다른 타깃 노드 수.
    out_deg: Vec<u16>,
    /// 단어 ID → 해당 단어의 key_out 노드.
    key_out_of: Vec<u16>,
    /// 단어 ID → 해당 단어의 소스 노드 (중복 제거, 정렬).
    word_sources: Vec<Vec<u16>>,
}

impl Graph {
    /// 사전에서 그래프를 구축한다.
    pub fn build(dict: &Dict) -> Self {
        let n = NODE_SPACE as usize;
        let mut out_words: Vec<HashSet<u32>> = (0..n).map(|_| HashSet::new()).collect::<Vec<_>>();
        let mut targets: Vec<HashSet<u16>> = (0..n).map(|_| HashSet::new()).collect::<Vec<_>>();
        let mut rev: Vec<HashSet<u16>> = (0..n).map(|_| HashSet::new()).collect::<Vec<_>>();
        let mut key_out_of: Vec<u16> = vec![0; dict.len()];
        let mut word_sources: Vec<Vec<u16>> = vec![Vec::new(); dict.len()];

        for w in &dict.words {
            key_out_of[w.id as usize] = w.key.key_out as u16;
            let srcs = sources_of(&w.key);
            word_sources[w.id as usize] = srcs
                .iter()
                .copied()
                .map(|s| s as u16)
                .collect::<Vec<u16>>();
            for src in srcs {
                let si = src as usize;
                out_words[si].insert(w.id);
                targets[si].insert(w.key.key_out as u16);
                rev[w.key.key_out as usize].insert(src as u16);
            }
        }

        let out_words_f = out_words
            .iter()
            .map(|s| {
                let mut v = s.iter().copied().collect::<Vec<u32>>();
                v.sort();
                v
            })
            .collect::<Vec<Vec<u32>>>();
        let out_neighbors_f = targets
            .iter()
            .map(|s| {
                let mut v = s.iter().copied().collect::<Vec<u16>>();
                v.sort();
                v
            })
            .collect::<Vec<Vec<u16>>>();
        let rev_f = rev
            .iter()
            .map(|s| {
                let mut v = s.iter().copied().collect::<Vec<u16>>();
                v.sort();
                v
            })
            .collect::<Vec<Vec<u16>>>();
        let out_deg = out_neighbors_f
            .iter()
            .map(|v| v.len() as u16)
            .collect::<Vec<u16>>();

        Self {
            out_words: out_words_f,
            out_neighbors: out_neighbors_f,
            rev: rev_f,
            out_deg,
            key_out_of,
            word_sources,
        }
    }

    /// 사전 단어 수.
    pub fn word_count(&self) -> u32 {
        self.word_sources.len() as u32
    }

    /// 노드에서 플레이 가능한 단어 ID.
    pub fn out_words(&self, node: u32) -> &[u32] {
        &self.out_words[node as usize]
    }

    /// 단어 ID → key_out 노드.
    pub fn key_out_of(&self, word_id: u32) -> u16 {
        self.key_out_of[word_id as usize]
    }

    /// 단어 ID → 소스 노드 (중복 제거, 정렬).
    pub fn word_sources(&self, word_id: u32) -> &[u16] {
        &self.word_sources[word_id as usize]
    }

    /// 노드의 서로 다른 타깃 노드 (정렬).
    pub fn out_neighbors(&self, node: u32) -> &[u16] {
        &self.out_neighbors[node as usize]
    }

    /// 노드로 도달하는 소스 노드 (역방향, 정렬).
    pub fn predecessors(&self, node: u32) -> &[u16] {
        &self.rev[node as usize]
    }

    /// 노드의 서로 다른 타깃 노드 수 (방어 단어 다양성).
    pub fn out_deg(&self, node: u32) -> u32 {
        self.out_deg[node as usize] as u32
    }
}

/// 단어가 플레이 가능한 소스 노드 집합.
///
/// - 자음 이음: `key_in_cho` 로 시작하는 단어를 플레이할 수 있는 모든 끝 대표음
///   자음 노드 (`INV_ALLOWED_INITS`).
/// - 음절 이음: 첫 음절이 개음절이면 해당 음절 노드에서도 플레이 가능.
pub fn sources_of(key: &WordKey) -> Vec<u32> {
    let mut out: Vec<u32> = inv_allowed_inits(key.key_in_cho).to_vec();
    if is_syllable(key.key_in) {
        out.push(key.key_in);
        out.sort();
    }
    out
}

/// 테스트용: 노드가 플레이 가능한지.
pub fn is_syllable_node(key: u32) -> bool {
    is_syllable(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use guel_dict::DictBuilder;
    use guel_kor::chain::{syllable_node, CONSONANT_BASE};

    fn graph_of(words: &[&str]) -> Graph {
        let mut b = DictBuilder::new();
        for w in words {
            b.add(w);
        }
        let d = b.build();
        Graph::build(&d)
    }

    #[test]
    fn syllable_node_edges() {
        // 바나나(ㅂ..나) 만 존재: '나' 노드에서 플레이 가능 단어 없음
        let g = graph_of(&["바나나"]);
        let na = syllable_node(2, 0); // '나'
        assert_eq!(g.out_words(na).len(), 0);
        // '바' 노드에서 플레이 가능 (바나나.key_in = '바')
        let ba = syllable_node(7, 0); // '바'
        assert_eq!(g.out_words(ba).len(), 1);
    }

    #[test]
    fn consonant_sources_include_duseum_sets() {
        // 시작 ㄴ(2) 단어 '나무': ㄴ-끝(2), ㄹ-끝(5) 자음과 '나' 음절에서 모두 플레이 가능
        let g = graph_of(&["나무"]);
        let nieun = CONSONANT_BASE + 2;
        let rieul = CONSONANT_BASE + 5;
        assert_eq!(g.out_words(nieun).len(), 1);
        assert_eq!(g.out_words(rieul).len(), 1);
        let na = syllable_node(2, 0); // '나'
        assert_eq!(g.out_words(na).len(), 1);
    }

    #[test]
    fn out_deg_counts_distinct_targets() {
        // 사과(사..과): '사' 노드, 타깃 '과'(ㄱ+ㅘ) 음절 노드
        let g = graph_of(&["사과"]);
        let sa = syllable_node(9, 0); // '사'
        assert_eq!(g.out_deg(sa), 1);
        // '과' 노드의 역방향 소스 = [ㅅ(9), '사']
        let gwa = syllable_node(0, 9); // '과' = ㄱ+ㅘ(jung 9)
        assert_eq!(g.predecessors(gwa), &[9, sa as u16]);
    }

    #[test]
    fn chain_dusuem_targets() {
        // 라면(ㄹ..ㄴ...): '라' 개음절 + ㄹ 초성
        let g = graph_of(&["라면"]);
        let ra = syllable_node(5, 0); // '라'
        assert_eq!(g.out_words(ra).len(), 1);
        // ㄹ 자음 노드에도 '라면' 포함 (key_in_cho = ㄹ)
        assert_eq!(g.out_words(CONSONANT_BASE + 5).len(), 1);
    }
}
