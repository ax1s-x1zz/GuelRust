//! 사전 단어 및 정규화.
//!
//! 원시 소스의 단어를 걸러 canonical `Word` 로 변환한다. 필터 규칙:
//! - NFC 정규화(기본), 한글 완성형 음절만 허용
//! - 길이 ≥ 2, 이음 키(`key_in`/`key_out`) 추출 가능해야 함
//! - 중복(동일 문자열) 제거

use guel_kor::chain::{word_key, WordKey};
use std::collections::HashSet;

/// 정규화된 단어 1건.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    /// 사전 내 유일 ID.
    pub id: u32,
    /// 단어 문자열 (정규화 완료).
    pub text: String,
    /// 이음 키.
    pub key: WordKey,
}

/// 원시 사전 소스 트레이트.
///
/// 표준국어대사전 CSV, 우리말샘 JSON, 단순 줄 단위 텍스트 등 여러 소스를
/// 이 트레이트로 추상화한다.
pub trait WordSource {
    /// 다음 원시 단어를 반환한다. 종료 시 `None`.
    fn next_word(&mut self) -> Option<String>;
}

/// 줄 단위(1줄 1단어) 텍스트 소스.
pub struct LineSource {
    lines: Vec<String>,
    idx: usize,
}

impl LineSource {
    pub fn new(text: &str) -> Self {
        Self {
            lines: text.split_whitespace().map(|s| s.to_string()).collect(),
            idx: 0,
        }
    }
}

impl WordSource for LineSource {
    fn next_word(&mut self) -> Option<String> {
        while self.idx < self.lines.len() {
            let w = &self.lines[self.idx];
            self.idx += 1;
            if is_acceptable(w) {
                return Some(w.clone());
            }
        }
        None
    }
}

/// 사전 빌더 — 소스에서 정규화된 단어 집합을 만든다.
pub struct DictBuilder {
    words: Vec<(u32, String)>,
    seen: HashSet<String>,
}

impl DictBuilder {
    pub fn new() -> Self {
        Self {
            words: Vec::new(),
            seen: HashSet::new(),
        }
    }

    /// 소스 전체를 소비해 단어를 추가한다.
    pub fn feed(&mut self, src: &mut impl WordSource) -> u32 {
        let mut added = 0;
        while let Some(raw) = src.next_word() {
            if self.seen.insert(raw.clone()) {
                self.words.push((self.words.len() as u32, raw));
                added += 1;
            }
        }
        added
    }

    /// 빌더에 단어를 직접 추가한다. 이미 있으면 무시.
    pub fn add(&mut self, raw: &str) -> bool {
        if !is_acceptable(raw) {
            return false;
        }
        if self.seen.insert(raw.to_string()) {
            self.words.push((self.words.len() as u32, raw.to_string()));
            true
        } else {
            false
        }
    }

    pub fn build(&self) -> Dict {
        let words = self
            .words
            .iter()
            .map(|(id, text)| Word {
                id: *id,
                text: text.clone(),
                key: word_key(text).unwrap(),
            })
            .collect();
        Dict { words }
    }
}

impl Default for DictBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 구축 완료된 사전.
pub struct Dict {
    pub words: Vec<Word>,
}

impl Dict {
    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub fn word(&self, id: u32) -> &Word {
        &self.words[id as usize]
    }
}

/// 사전 수용 기준(acceptability) 판정.
///
/// - 한글 완성형 음절만 포함 (공백·숫자·외래문자·제어문자 제거)
/// - 길이 ≥ 2
/// - 이음 키 추출 가능
pub fn is_acceptable(word: &str) -> bool {
    if word.chars().count() < 2 {
        return false;
    }
    let all_hangul = word.chars().all(guel_kor::jamo::is_syllable);
    if !all_hangul {
        return false;
    }
    word_key(word).is_some()
}

/// NFC 정규화 (기본 구현).
///
/// 완전한 NFC 합성은 자모 결합 조합까지 수행한다. 현재는 완성형 한글
/// 블록(U+AC00..U+D7A3) 밖의 자모 문자를 제거하는 최소 정규화를 제공한다.
/// (추후 고급 정규화는 별도 모듈로 분리)
pub fn normalize_nfc(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            let cp = *c as u32;
            (0xAC00..=0xD7A3).contains(&cp) || c.is_whitespace()
        })
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(words: &[&str]) -> Dict {
        let mut b = DictBuilder::new();
        for w in words {
            b.add(w);
        }
        b.build()
    }

    #[test]
    fn accepts_hangul_words_only() {
        assert!(is_acceptable("바나나"));
        assert!(is_acceptable("사과"));
        assert!(!is_acceptable("a"));
        assert!(!is_acceptable("banana"));
        assert!(!is_acceptable("나")); // 길이 1
        assert!(!is_acceptable("나 a")); // 공백 포함
        assert!(!is_acceptable(""));
    }

    #[test]
    fn dedupe_and_ids() {
        let d = build(&["사과", "사과", "바나나", "나"]);
        assert_eq!(d.len(), 2);
        assert_eq!(d.word(0).text, "사과");
        assert_eq!(d.word(1).text, "바나나");
    }

    #[test]
    fn keys_assigned() {
        let d = build(&["사과", "바나나"]);
        assert_eq!(d.word(0).key.key_in, 9); // ㅅ
        assert_eq!(d.word(0).key.key_out, 0); // ㄱ (개음절 과)
        assert_eq!(d.word(1).key.key_in, 7); // ㅂ
        assert_eq!(d.word(1).key.key_out, 2); // ㄴ (개음절 나)
    }

    #[test]
    fn line_source_feeds() {
        let mut src = LineSource::new("사과\n바나나\n\n포도\n");
        let mut b = DictBuilder::new();
        assert_eq!(b.feed(&mut src), 3);
        assert_eq!(b.build().len(), 3);
    }

    #[test]
    fn normalize_removes_jamo() {
        let n = normalize_nfc("ㅎ바나나");
        assert_eq!(n, "바나나");
    }
}
