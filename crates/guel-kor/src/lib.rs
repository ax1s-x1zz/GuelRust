//! `guel-kor` — 한글 자모 분해/결합, 대표음(겹받침), 두음법칙 규칙 코덱.
//!
//! 구엘룰 엔진의 음운 규칙 레이어. 모든 로직은 0-dep 산술 기반이며,
//! PRD §4.3.1 의 두음법칙 및 대표음 매핑을 구현한다.

pub mod duseum;
pub mod jamo;
pub mod representative;

pub use duseum::{allowed_inits, can_chain};
pub use jamo::{compose, decompose, Jamo};
pub use representative::representative_jong;

/// 단어의 이음 키(`key_in`/`key_out`) 산출용 최상위 API.
///
/// [`chain::WordKey`] 와 결합해 사용한다.
pub mod chain {
    use crate::jamo;
    use crate::representative;

    /// 초성-대응 노드 공간에서 자모 수.
    pub const CHO_N: u32 = 19;
    /// 중성 수.
    pub const JUNG_N: u32 = 21;

    /// 노드 공간: 자모(자음) 키 베이스 (0..18).
    pub const CONSONANT_BASE: u32 = 0;
    /// 노드 공간: 음절(초성+중성) 키 베이스 (100..498).
    pub const SYLLABLE_BASE: u32 = 100;

    /// 단어의 시작소리/끝소리 키.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct WordKey {
        /// 시작소리 키(노드 공간):
        /// - 첫 음절이 받침 있음 → `CONSONANT_BASE + 초성`
        /// - 첫 음절이 받침 없음 → `SYLLABLE_BASE + 초성*중성수 + 중성`
        pub key_in: u32,
        /// 첫 음절의 초성 (두음법칙 허용 집합 판정용).
        pub key_in_cho: u32,
        /// 끝소리 키(노드 공간):
        /// - 마지막 음절이 받침 있음 → `CONSONANT_BASE + 대표음 초성`
        /// - 마지막 음절이 받침 없음 → `SYLLABLE_BASE + 초성*중성수 + 중성`
        pub key_out: u32,
        /// 마지막 음절이 받침 없음(개음절)인지.
        pub open_final: bool,
    }

    /// 초성·중성 인덱스로 음절 노드 키를 만든다.
    #[inline]
    pub fn syllable_node(cho: u32, jung: u32) -> u32 {
        SYLLABLE_BASE + cho * JUNG_N + jung
    }

    /// 노드 키가 자음(받침 이음)인지.
    #[inline]
    pub fn is_consonant_node(key: u32) -> bool {
        key < CONSONANT_BASE + CHO_N
    }

    /// 단어에서 이음 키를 산출한다.
    pub fn word_key(word: &str) -> Option<WordKey> {
        let first = jamo::first_syllable(word)?;
        let last = jamo::last_syllable(word)?;
        let key_in = if first.has_jong() {
            CONSONANT_BASE + first.cho
        } else {
            syllable_node(first.cho, first.jung)
        };
        let key_out = if last.has_jong() {
            let rep_jong = representative::representative_jong(last.jong);
            CONSONANT_BASE + jamo::JONG_TO_CHO[rep_jong as usize]
        } else {
            syllable_node(last.cho, last.jung)
        };
        Some(WordKey {
            key_in,
            key_in_cho: first.cho,
            key_out,
            open_final: !last.has_jong(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::chain::{syllable_node, word_key, CONSONANT_BASE};

    #[test]
    fn basic_chain_keys() {
        // 바나나: 시작 ㅂ(7) 받침없음 -> 음절 '바' 노드
        let k = word_key("바나나").unwrap();
        assert_eq!(k.key_in_cho, 7); // ㅂ
        assert_eq!(k.key_in, syllable_node(7, 0)); // '바' (ㅂ+ㅏ)
                                                   // 끝 '나' 개음절 -> 음절 '나' 노드
        assert_eq!(k.key_out, syllable_node(2, 0)); // '나' (ㄴ+ㅏ)
        assert!(k.open_final);
    }

    #[test]
    fn consonant_final_uses_representative() {
        // 닭: 시작 ㄷ(3) 받침있음 -> 자음 ㄷ 노드, 끝 ㄺ -> 대표음 ㄱ(초성 0)
        let k = word_key("닭").unwrap();
        assert_eq!(k.key_in, CONSONANT_BASE + 3); // ㄷ
        assert_eq!(k.key_in_cho, 3);
        assert_eq!(k.key_out, CONSONANT_BASE + 0); // ㄱ
        assert!(!k.open_final);
    }

    #[test]
    fn open_syllable_in_is_syllable_node() {
        // 가방: 시작 '가'(ㄱ+ㅏ) 받침없음 -> 음절 노드
        let k = word_key("가방").unwrap();
        assert_eq!(k.key_in, syllable_node(0, 0)); // '가'
        assert_eq!(k.key_in_cho, 0); // ㄱ
        assert!(!k.open_final); // 끝 '방' 받침 있음(ㅇ)
        assert_eq!(k.key_out, CONSONANT_BASE + 11); // ㅇ
    }
}
