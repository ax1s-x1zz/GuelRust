//! 스왑(단어 뺏기) — 후공 권한 모델.
//!
//! 구엘룰 특수 규칙(PRD §4.3.2):
//! - 선공이 단어 `X`(끝소리 `t`)를 플레이하면 후공은 "스왑"을 선언해
//!   `X` 의 소유권을 뺏고, 턴을 선공에게 복귀시킨다 (선공이 `t` 에서 이어감).
//! - 스왑은 `t` 가 L0(한방)일 때 금지 — 한방 회피 불가.
//! - 스왑 회수는 룰 파라미터로 제한.

use crate::engine::Engine;
use crate::oracle::{is_loss, LevelTable};

/// 스왑 룰 파라미터.
#[derive(Debug, Clone, Copy)]
pub struct SwapRules {
    /// 게임당 최대 스왑 횟수.
    pub max_per_game: u8,
}

impl SwapRules {
    pub fn standard() -> Self {
        Self { max_per_game: 1 }
    }
}

/// 단어 소유권.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Us,
    Them,
}

/// 스왑을 포함한 게임 상태 래퍼.
pub struct Game<'g, 'e> {
    engine: &'e mut Engine<'g>,
    rules: SwapRules,
    swaps_used: u8,
    last_word: Option<u32>,
    last_owner: Option<Owner>,
}

impl<'g, 'e> Game<'g, 'e> {
    pub fn new(engine: &'e mut Engine<'g>, rules: SwapRules) -> Game<'g, 'e> {
        Self {
            engine,
            rules,
            swaps_used: 0,
            last_word: None,
            last_owner: None,
        }
    }

    /// 내가 단어를 플레이한다.
    pub fn play_our(&mut self, word_id: u32) {
        self.engine.play(word_id);
        self.last_word = Some(word_id);
        self.last_owner = Some(Owner::Us);
    }

    /// 상대가 단어를 플레이했다 (후공 입장에서 선공 수를 기록).
    pub fn on_their_move(&mut self, word_id: u32) {
        self.engine.play(word_id);
        self.last_word = Some(word_id);
        self.last_owner = Some(Owner::Them);
    }

    /// 마지막 단어의 끝소리 노드.
    pub fn current_node(&self) -> u32 {
        self.engine.current_key_out(self.last_word.unwrap())
    }

    /// 스왑 선언 가능 여부.
    ///
    /// - 스왑 회수 초과 시 불가
    /// - 마지막 단어가 없거나 상대 소유가 아니면 불가
    /// - 끝소리가 L0(한방)이면 불가
    pub fn can_swap(&self, levels: &LevelTable) -> bool {
        if self.swaps_used >= self.rules.max_per_game {
            return false;
        }
        if self.last_word.is_none() || self.last_owner != Some(Owner::Them) {
            return false;
        }
        let t = self.engine.current_key_out(self.last_word.unwrap());
        let lvl = levels.get(t);
        // 한방(L0) 회피 불가.
        lvl != 0
    }

    /// 스왑이 유리한지 (수심 최적 판단).
    ///
    /// 스왑 전후 잔여 그래프는 동일(단어는 어차피 소비됨)하므로, 문제는
    /// "누가 `t` 에서 먼저 두는가"다. `t` 가 L(선 이동 패배)이면 우리가
    /// 두면 지므로 → 상대에게 턴을 넘기는 스왑이 유리하다.
    pub fn swap_beneficial(&self, levels: &LevelTable) -> bool {
        if !self.can_swap(levels) {
            return false;
        }
        let t = self.engine.current_key_out(self.last_word.unwrap());
        let lvl = levels.get(t);
        // L → 스왑 (우리가 두면 패배), W/D → 유지.
        is_loss(lvl)
    }

    /// 스왑 실행. 소유권 이전 + 턴 복귀를 상태에 반영한다.
    ///
    /// (엔진의 단어 소비는 이미 `on_their_move` 에서 이뤄져 있고, 소유권만
    /// 이전되므로 잔여 그래프는 변하지 않는다.)
    pub fn do_swap(&mut self) -> bool {
        if !self.can_swap(self.engine.residual_table()) {
            return false;
        }
        self.swaps_used += 1;
        self.last_owner = Some(Owner::Us);
        true
    }

    /// 남은 스왑 회수.
    pub fn swaps_left(&self) -> u8 {
        self.rules.max_per_game.saturating_sub(self.swaps_used)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use crate::oracle::retrograde;
    use guel_dict::DictBuilder;

    fn graph_of(words: &[&str]) -> Graph {
        let mut b = DictBuilder::new();
        for w in words {
            b.add(w);
        }
        let d = b.build();
        Graph::build(&d)
    }

    #[test]
    fn swap_beneficial_on_loss_node() {
        let g = graph_of(&["바나나", "가구"]);
        let o = retrograde(&g);
        let mut e = Engine::new(&g, &o);
        let mut game = Game::new(&mut e, SwapRules::standard());
        // '가구'(id 1) 플레이 → t='구' (W1): 우리가 두면 이김 → 스왑 불필요.
        game.on_their_move(1);
        let levels = game.engine.residual_table();
        assert!(!game.swap_beneficial(levels));
    }

    #[test]
    fn swap_forbidden_on_l0() {
        let g = graph_of(&["바나나", "가구"]);
        let o = retrograde(&g);
        let mut e = Engine::new(&g, &o);
        let mut game = Game::new(&mut e, SwapRules::standard());
        // '바나나'(id 0) 플레이 → t='나'(L0): 스왑 금지(한방 회피 불가).
        game.on_their_move(0);
        assert!(!game.can_swap(game.engine.residual_table()));
    }

    #[test]
    fn swap_consumes_token_and_owned_by_us() {
        let g = graph_of(&["가구", "구름", "구슬"]);
        let o = retrograde(&g);
        let mut e = Engine::new(&g, &o);
        let mut game = Game::new(&mut e, SwapRules::standard());
        game.on_their_move(0); // 가구 → t='구'
        let levels = game.engine.residual_table();
        if game.can_swap(levels) {
            assert!(game.do_swap());
            assert_eq!(game.swaps_left(), 0);
            assert_eq!(game.last_owner, Some(Owner::Us));
            // 연속 스왑 불가
            assert!(!game.do_swap());
        }
    }

    #[test]
    fn max_swaps_respected() {
        let g = graph_of(&["가구", "구름", "구슬"]);
        let o = retrograde(&g);
        let mut e = Engine::new(&g, &o);
        let game = Game::new(&mut e, SwapRules { max_per_game: 0 });
        let levels = game.engine.residual_table();
        assert!(!game.can_swap(levels));
    }
}