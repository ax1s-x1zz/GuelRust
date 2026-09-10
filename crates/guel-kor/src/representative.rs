//! 겹받침/받침의 대표음(표준 발음) 매핑.
//!
//! 끝말잇기 이음 키 `key_out` 을 산출할 때 마지막 음절의 종성을 대표음으로
//! 단순화한다. 표준 규정을 기본값으로 하되 `소리/겹받침` 예외를 허용한다.

use crate::jamo::JONGSEONG_N;

/// 종성 인덱스 → 대표음 종성 인덱스 테이블 (표준 대표음 기준).
///
/// PRD §4.3.1 의 매핑을 따른다:
/// `ㄳ→ㄱ, ㄵ→ㄴ, ㄺ→ㄱ, ㄼ→ㅂ, ㄽ→ㄹ, ㄾ→ㄹ, ㅄ→ㅂ, ㄻ→ㅁ, ㄿ→ㅍ, ㄲ→ㄱ, ㅆ→ㅆ, ㄶ→ㄴ, ㅀ→ㄹ`
/// 그 외 단일 받침(`ㄱ,ㄴ,ㄷ,ㄹ,ㅁ,ㅂ,ㅅ,ㅇ,ㅈ,ㅊ,ㅋ,ㅌ,ㅍ,ㅎ`)은 표준 발음 대표음으로
/// (`ㅅ→ㄷ, ㅈ→ㄷ, ㅊ→ㄷ, ㅋ→ㄱ, ㅌ→ㄷ, ㅍ→ㅂ, ㅎ→ㄷ`).
pub const REPRESENTATIVE: [u8; JONGSEONG_N as usize] = [
    // 0: 없음
    0, // 1 ㄱ
    1, // 2 ㄲ
    1, // 3 ㄳ
    1, // 4 ㄴ
    4, // 5 ㄵ
    4, // 6 ㄶ
    4, // 7 ㄷ
    7, // 8 ㄹ
    8, // 9 ㄺ
    1, // 10 ㄻ
    16, // 11 ㄼ
    17, // 12 ㄽ
    8, // 13 ㄾ
    8, // 14 ㄿ
    26, // 15 ㅀ
    8, // 16 ㅁ
    16, // 17 ㅂ
    17, // 18 ㅄ
    17, // 19 ㅅ
    7, // 20 ㅆ
    20, // 21 ㅇ
    21, // 22 ㅈ
    7, // 23 ㅊ
    7, // 24 ㅋ
    1, // 25 ㅌ
    7, // 26 ㅍ
    17, // 27 ㅎ
    7,
];

/// 종성 인덱스를 대표음 종성 인덱스로 변환한다.
#[inline]
pub fn representative_jong(jong: u32) -> u32 {
    debug_assert!(jong < JONGSEONG_N);
    REPRESENTATIVE[jong as usize] as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_finals_to_representative() {
        // ㄳ(3) -> ㄱ(1)
        assert_eq!(representative_jong(3), 1);
        // ㄺ(9) -> ㄱ(1)
        assert_eq!(representative_jong(9), 1);
        // ㄵ(5) -> ㄴ(4)
        assert_eq!(representative_jong(5), 4);
        // ㄻ(10) -> ㅁ(16)
        assert_eq!(representative_jong(10), 16);
        // ㄼ(11) -> ㅂ(17)
        assert_eq!(representative_jong(11), 17);
        // ㄾ(13) -> ㄹ(8)
        assert_eq!(representative_jong(13), 8);
        // ㄿ(14) -> ㅍ(26)
        assert_eq!(representative_jong(14), 26);
        // ㅀ(15) -> ㄹ(8)
        assert_eq!(representative_jong(15), 8);
    }

    #[test]
    fn single_finals() {
        // ㄱ -> ㄱ
        assert_eq!(representative_jong(1), 1);
        // ㄴ -> ㄴ
        assert_eq!(representative_jong(4), 4);
        // ㄷ -> ㄷ
        assert_eq!(representative_jong(7), 7);
        // ㅁ -> ㅁ
        assert_eq!(representative_jong(16), 16);
        // ㅇ -> ㅇ
        assert_eq!(representative_jong(21), 21);
        // ㅅ -> ㄷ
        assert_eq!(representative_jong(19), 7);
        // ㅎ -> ㄷ
        assert_eq!(representative_jong(27), 7);
    }
}
