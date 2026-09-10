//! 두음법칙(word-initial sound law) 허용 초성 규칙.
//!
//! 이전 단어의 끝 대표음 `c` 에 대해, 다음 단어가 시작할 수 있는 초성 집합
//! `A(c)` 를 정의한다. PRD §4.3.1 을 따른다:
//! - `A(ㄹ) ⊇ {ㄹ, ㄴ, ㅇ}`
//! - `A(ㄴ) ⊇ {ㄴ, ㅇ}`
//! - 그 외 종성은 자기 자신.
//!
//! 초성 인덱스는 `crate::jamo::CHOSEONG` 기준 (0..19).

use crate::jamo::CHOSEONG_N;

/// 이전 끝 대표음(초성 인덱스로 표현) → 다음 단어가 시작 가능한 초성 집합.
/// 단일 슬라이스(인접 목록)로 저장한다.
pub const ALLOWED_INITS: [&[u32]; CHOSEONG_N as usize] = [
    // ㄱ(0)
    &[0],
    // ㄲ(1)
    &[1],
    // ㄴ(2) : 두음법칙 ㄴ→ㅇ
    &[2, 11],
    // ㄷ(3)
    &[3],
    // ㄸ(4)
    &[4],
    // ㄹ(5) : 두음법칙 ㄹ→ㄴ/ㅇ
    &[5, 2, 11],
    // ㅁ(6)
    &[6],
    // ㅂ(7)
    &[7],
    // ㅃ(8)
    &[8],
    // ㅅ(9)
    &[9],
    // ㅆ(10)
    &[10],
    // ㅇ(11) : 모음 시작
    &[11],
    // ㅈ(12)
    &[12],
    // ㅉ(13)
    &[13],
    // ㅊ(14)
    &[14],
    // ㅋ(15)
    &[15],
    // ㅌ(16)
    &[16],
    // ㅍ(17)
    &[17],
    // ㅎ(18)
    &[18],
];

/// 다음 단어의 첫 초성 `cho` 가 이전 끝 대표음 `rep` 의 허용 집합에 속하는지.
#[inline]
pub fn can_chain(rep: u32, cho: u32) -> bool {
    debug_assert!(rep < CHOSEONG_N && cho < CHOSEONG_N);
    ALLOWED_INITS[rep as usize].contains(&cho)
}

/// 이전 끝 대표음 `rep` 에 대해 시작 가능한 초성 집합.
pub fn allowed_inits(rep: u32) -> &'static [u32] {
    debug_assert!(rep < CHOSEONG_N);
    ALLOWED_INITS[rep as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rieul_ends_accept_rieul_nieun_ieung() {
        // ㄹ(5) -> {ㄹ(5), ㄴ(2), ㅇ(11)}
        assert!(can_chain(5, 5));
        assert!(can_chain(5, 2));
        assert!(can_chain(5, 11));
        assert!(!can_chain(5, 0)); // ㄱ 불가
    }

    #[test]
    fn nieun_ends_accept_nieun_ieung() {
        assert!(can_chain(2, 2));
        assert!(can_chain(2, 11));
        assert!(!can_chain(2, 5)); // ㄹ 불가
    }

    #[test]
    fn plain_consonant_only_itself() {
        assert!(can_chain(0, 0)); // ㄱ->ㄱ
        assert!(!can_chain(0, 1)); // ㄱ->ㄲ 불가
        assert!(can_chain(7, 7)); // ㅂ->ㅂ
    }

    #[test]
    fn ieung_ends_accept_ieung() {
        assert!(can_chain(11, 11));
        assert!(!can_chain(11, 0));
    }
}
