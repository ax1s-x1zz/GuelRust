//! 게임 노드 공간 — 자음/음절 노드 키.

use guel_kor::chain::{CHO_N, CONSONANT_BASE, SYLLABLE_BASE};

/// 노드 공간 상한 (자음 19 + 음절 399).
pub const NODE_SPACE: u32 = CHO_N + 399;

/// 노드 키가 자음(받침 이음) 노드인지.
#[inline]
pub fn is_consonant(key: u32) -> bool {
    key < CONSONANT_BASE + CHO_N
}

/// 노드 키가 음절(개음절 이음) 노드인지.
#[inline]
pub fn is_syllable(key: u32) -> bool {
    (SYLLABLE_BASE..SYLLABLE_BASE + 399).contains(&key)
}

/// 모든 자음 노드 키.
pub fn all_consonants() -> [u32; 19] {
    [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use guel_kor::chain::syllable_node;

    #[test]
    fn consonant_and_syllable_disjoint() {
        assert!(is_consonant(CONSONANT_BASE + 5));
        assert!(!is_consonant(syllable_node(0, 0)));
        assert!(is_syllable(syllable_node(18, 20)));
        assert!(!is_syllable(CONSONANT_BASE + 3));
    }

    #[test]
    fn node_space_bounds() {
        assert_eq!(NODE_SPACE, 19 + 399);
        assert!(syllable_node(18, 20) < SYLLABLE_BASE + 399);
    }
}
