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

    /// 단어의 시작소리/끝소리 키.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct WordKey {
        /// 시작소리 키: 첫 음절의 초성 인덱스.
        pub key_in: u32,
        /// 끝소리 키: 마지막 음절의 대표음 종성에 대응하는 초성 인덱스.
        pub key_out: u32,
        /// 마지막 음절이 받침 없음(개음절)인지.
        pub open_final: bool,
    }

    /// 단어에서 이음 키를 산출한다.
    ///
    /// `key_in` 은 첫 음절의 초성, `key_out` 은 마지막 음절의 대표음 종성에
    /// 대응하는 초성 인덱스(받침 없으면 해당 음절의 초성) 를 반환한다.
    /// 둘 다 초성 인덱스 공간이라 `key_out → key_in` 이음 비교가 가능하다.
    pub fn word_key(word: &str) -> Option<WordKey> {
        let first = jamo::first_syllable(word)?;
        let last = jamo::last_syllable(word)?;
        let key_out = if last.has_jong() {
            let rep_jong = representative::representative_jong(last.jong);
            jamo::JONG_TO_CHO[rep_jong as usize]
        } else {
            last.cho
        };
        Some(WordKey {
            key_in: first.cho,
            key_out,
            open_final: !last.has_jong(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::chain::word_key;

    #[test]
    fn basic_chain_keys() {
        // 바나나: 시작 ㅂ(7), 끝 ㄴ(4) 받침없음 -> key_out = 나의 초성 ㄴ(2)? 아니, 나는 받침없음이므로 초성 ㄴ
        // 마지막 음절 '나' 의 초성 ㄴ = 2
        let k = word_key("바나나").unwrap();
        assert_eq!(k.key_in, 7); // ㅂ
        assert_eq!(k.key_out, 2); // ㄴ
        assert!(k.open_final);
    }

    #[test]
    fn consonant_final_uses_representative() {
        // 사과: 시작 ㅅ(9), 끝 '과' 는 개음절이므로 초성 ㄱ(0)
        let k = word_key("사과").unwrap();
        assert_eq!(k.key_in, 9);
        assert_eq!(k.key_out, 0); // ㄱ
        assert!(k.open_final);
    }

    #[test]
    fn double_final_chain() {
        // 닭: 시작 ㄷ(3), 끝 ㄺ -> 대표음 ㄱ(0)
        let k = word_key("닭").unwrap();
        assert_eq!(k.key_in, 3);
        assert_eq!(k.key_out, 0); // ㄱ
        assert!(!k.open_final);
    }
}
