//! 한글 자모 분해/결합 코덱 (0-dep, 산술 기반).
//!
//! 완성형 한글 음절 `U+AC00..U+D7A3` 을 초성/중성/종성 인덱스로 분해하고,
//! 역으로 결합한다. 모든 함수는 `const` 친화적으로 구현하여 핫 경로에서도
//! 추가 의존성 없이 사용할 수 있다.

/// 완성형 한글 블록 시작 코드 포인트 (가).
pub const HANGUL_BASE: u32 = 0xAC00;
/// 완성형 한글 블록 끝 (힣).
pub const HANGUL_END: u32 = 0xD7A3;

/// 초성 개수.
pub const CHOSEONG_N: u32 = 19;
/// 중성 개수.
pub const JUNGSEONG_N: u32 = 21;
/// 종성 개수 (0=받침 없음 포함).
pub const JONGSEONG_N: u32 = 28;

/// 종성이 하나일 때의 결합 단위.
pub const SYLLABLE_UNIT: u32 = JUNGSEONG_N * JONGSEONG_N;

/// 초성 테이블 (전통 자모 순서).
pub const CHOSEONG: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ',
    'ㅌ', 'ㅍ', 'ㅎ',
];

/// 중성 테이블.
pub const JUNGSEONG: [char; 21] = [
    'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ', 'ㅞ',
    'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ',
];

/// 종성 테이블 (첫 항목은 받침 없음).
pub const JONGSEONG: [char; 28] = [
    '\0', 'ㄱ', 'ㄲ', 'ㄳ', 'ㄴ', 'ㄵ', 'ㄶ', 'ㄷ', 'ㄹ', 'ㄺ', 'ㄻ', 'ㄼ', 'ㄽ', 'ㄾ', 'ㄿ', 'ㅀ',
    'ㅁ', 'ㅂ', 'ㅄ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];

/// 종성 인덱스 → 같은 자모의 초성 인덱스.
///
/// 종성 테이블과 초성 테이블의 인덱스 공간이 서로 달라(`ㄱ` 종성=1, 초성=0),
/// 이음 키 비교를 위해 동일한 초성 인덱스 공간으로 정규화한다.
pub const JONG_TO_CHO: [u32; JONGSEONG_N as usize] = [
    0,  // 0 없음 -> ㅇ(모음 시작으로 간주하지 않음)
    0,  // 1 ㄱ
    1,  // 2 ㄲ
    0,  // 3 ㄳ
    2,  // 4 ㄴ
    2,  // 5 ㄵ
    2,  // 6 ㄶ
    3,  // 7 ㄷ
    5,  // 8 ㄹ
    0,  // 9 ㄺ
    6,  // 10 ㄻ
    7,  // 11 ㄼ
    5,  // 12 ㄽ
    5,  // 13 ㄾ
    17, // 14 ㄿ
    5,  // 15 ㅀ
    6,  // 16 ㅁ
    7,  // 17 ㅂ
    7,  // 18 ㅄ
    9,  // 19 ㅅ
    10, // 20 ㅆ
    11, // 21 ㅇ
    3,  // 22 ㅈ
    3,  // 23 ㅊ
    0,  // 24 ㅋ
    3,  // 25 ㅌ
    17, // 26 ㅍ
    3,  // 27 ㅎ
];

/// 단일 음절 구조.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Jamo {
    pub cho: u32,
    pub jung: u32,
    pub jong: u32,
}

impl Jamo {
    /// `0`이면 받침 없음.
    pub fn has_jong(&self) -> bool {
        self.jong != 0
    }
}

/// 코드 포인트가 완성형 한글 음절인지.
#[inline]
pub fn is_syllable(c: char) -> bool {
    let c = c as u32;
    (HANGUL_BASE..=HANGUL_END).contains(&c)
}

/// 완성형 한글 음절을 자모 인덱스로 분해한다.
///
/// 완성형 한글이 아닌 문자는 `None` 을 반환한다.
#[inline]
pub fn decompose(c: char) -> Option<Jamo> {
    let cp = c as u32;
    if !is_syllable(c) {
        return None;
    }
    let s = cp - HANGUL_BASE;
    Some(Jamo {
        cho: s / SYLLABLE_UNIT,
        jung: (s % SYLLABLE_UNIT) / JONGSEONG_N,
        jong: s % JONGSEONG_N,
    })
}

/// 자모 인덱스로 완성형 한글 음절을 결합한다.
///
/// `jung < JUNGSEONG_N`, `jong < JONGSEONG_N` 이어야 한다. 범위를 벗어나면 `None`.
#[inline]
pub fn compose(cho: u32, jung: u32, jong: u32) -> Option<char> {
    if cho >= CHOSEONG_N || jung >= JUNGSEONG_N || jong >= JONGSEONG_N {
        return None;
    }
    let c = HANGUL_BASE + cho * SYLLABLE_UNIT + jung * JONGSEONG_N + jong;
    char::from_u32(c)
}

/// 문자열에서 각 음절의 초성만 추출한다.
pub fn initial_cho_of(s: &str) -> Vec<u32> {
    s.chars()
        .filter_map(|c| decompose(c).map(|j| j.cho))
        .collect()
}

/// 문자열에서 첫 음절의 자모를 반환한다 (비한글 첫 글자는 `None`).
pub fn first_syllable(s: &str) -> Option<Jamo> {
    s.chars().find_map(decompose)
}

/// 문자열에서 마지막 음절의 자모를 반환한다 (비한글 마지막 글자는 `None`).
pub fn last_syllable(s: &str) -> Option<Jamo> {
    s.chars().filter_map(decompose).next_back()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompose_가() {
        let j = decompose('가').unwrap();
        assert_eq!((j.cho, j.jung, j.jong), (0, 0, 0));
    }

    #[test]
    fn decompose_힣() {
        let j = decompose('힣').unwrap();
        assert_eq!((j.cho, j.jung, j.jong), (18, 20, 27));
    }

    #[test]
    fn decompose_일() {
        let j = decompose('일').unwrap();
        assert_eq!(j.cho, 11); // ㅇ
        assert_eq!(j.jung, 20); // ㅣ
        assert_eq!(j.jong, 8); // ㄹ
    }

    #[test]
    fn compose_roundtrip() {
        for c in CHOSEONG.iter().enumerate() {
            for j in 0..JUNGSEONG_N {
                for k in 0..JONGSEONG_N {
                    let cc = compose(c.0 as u32, j, k).unwrap();
                    let d = decompose(cc).unwrap();
                    assert_eq!((d.cho, d.jung, d.jong), (c.0 as u32, j, k));
                }
            }
        }
    }

    #[test]
    fn non_hangul_returns_none() {
        assert!(decompose('a').is_none());
        assert!(decompose('中').is_none());
    }

    #[test]
    fn first_and_last() {
        let f = first_syllable("바나나").unwrap();
        let l = last_syllable("바나나").unwrap();
        assert_eq!(f.cho, 7); // ㅂ
        assert_eq!(l.jong, 0); // 나: 받침 없음
    }

    #[test]
    fn last_of_banana_is_na() {
        let l = last_syllable("바나나").unwrap();
        assert_eq!((l.cho, l.jung, l.jong), (2, 0, 0)); // ㄴ ㅏ 받침없음
    }
}
