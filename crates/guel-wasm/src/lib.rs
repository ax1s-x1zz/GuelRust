//! `guel-wasm` — 구엘룰 WASM 바인딩.
//!
//! 브라우저(Web Worker)에서 대국 가능하도록 `Bot` 을 노출한다.
//! - 생성 시 사전 텍스트를 받아 그래프·정적 오라클을 프리컴파일한다.
//! - `play`/`best_move`/`classify` 등으로 대국 상태를 유지한다.

use guel_core::{Engine, Graph, Side, is_win, retrograde};
use guel_dict::{Dict, DictBuilder, LineSource};
use wasm_bindgen::prelude::wasm_bindgen;

/// 브라우저 대국 봇.
#[wasm_bindgen]
pub struct Bot {
    engine: Engine,
    dict: Dict,
}

#[wasm_bindgen]
impl Bot {
    /// 사전 텍스트(줄/공백 구분)로 봇을 구축한다.
    #[wasm_bindgen(constructor)]
    pub fn new(dict_text: &str) -> Bot {
        let mut b = DictBuilder::new();
        b.feed(&mut LineSource::new(dict_text.to_string()));
        let dict = b.build();
        let graph = Graph::build(&dict);
        let oracle = retrograde(&graph);
        let engine = Engine::new(graph, oracle);
        Bot { engine, dict }
    }

    /// 사전 단어 수.
    #[wasm_bindgen(getter)]
    pub fn word_count(&self) -> u32 {
        self.dict.len() as u32
    }

    /// 단어 ID의 텍스트.
    pub fn word_text(&self, word_id: u32) -> String {
        self.dict.word(word_id).text.clone()
    }

    /// 단어를 플레이한다.
    pub fn play(&mut self, word_id: u32) {
        self.engine.play(word_id);
    }

    /// 현재(잔여) 레벨 분류.
    pub fn classify(&self, node: u32) -> u8 {
        self.engine.classify(node)
    }

    /// 시작 노드의 필승 사이드 (0=선공, 1=후공, 2=어느 쪽).
    pub fn start_side(&self, node: u32) -> u8 {
        match self.engine.start_side(node) {
            Side::First => 0,
            Side::Second => 1,
            Side::Either => 2,
        }
    }

    /// 시작 노드가 선공 필승인지.
    pub fn is_winning(&self, node: u32) -> bool {
        is_win(self.engine.full_oracle().get(node))
    }

    /// 최선수 단어 ID (둘 곳 없으면 u32::MAX).
    pub fn best_move(&mut self, node: u32) -> u32 {
        let mv = self.engine.best_move(node);
        mv.map(|m| m.word_id).unwrap_or(u32::MAX)
    }

    /// 최선수와 함께 즉시 플레이까지 수행해 단어 ID를 반환 (없으면 u32::MAX).
    pub fn best_move_and_play(&mut self, node: u32) -> u32 {
        let mv = self.engine.best_move(node);
        if mv.is_none() {
            return u32::MAX;
        }
        let m = mv.unwrap();
        self.engine.play(m.word_id);
        m.word_id
    }
}