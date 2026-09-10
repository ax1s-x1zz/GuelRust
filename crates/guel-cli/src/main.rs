use guel_core::{Engine, Graph, Outcome, is_loss, is_win, retrograde, simulate_engine_vs_random, Side, NODE_SPACE};
use guel_dict::{Dict, DictBuilder, LineSource};
use guel_kor::chain::word_key;

/// 임베디드 샘플 사전 (파일 없이 즉시 실행용).
const SAMPLE_DICT: &str = "\
가구 구름 구슬 구두 두부 부엌 \
파리 리본 본보기 기본 기와 와이파이 \
해파리 소나무 무지개 개구리 가방 방문 \
문구 구경 경찰 나무 바나나 사과 사람 물고기 \
고래 래프팅 프린터 터널 널빤지 지갑 갑옷 옷장 \
장미 미로 로봇 봇틀 틀니 니트 트램 램프 프록시 시계";

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<String>>();
    if args.is_empty() {
        print_help();
        return;
    }
    let cmd = &args[0];
    if *cmd == "stats" {
        let (d, g) = load_dict(if args.len() > 1 { Some(args[1].clone()) } else { None });
        cmd_stats(&d, &g);
    } else if *cmd == "play" {
        // 인자: play [dict] [시작음]
        let (path, start) = if args.len() >= 3 {
            (Some(args[1].clone()), args[2].clone())
        } else if args.len() == 2 {
            (None, args[1].clone())
        } else {
            (None, String::new())
        };
        let (d, g) = load_dict(path);
        cmd_play(&d, &g, Some(start));
    } else if *cmd == "verify" {
        let (d, g) = load_dict(if args.len() > 1 { Some(args[1].clone()) } else { None });
        cmd_verify(&d, &g);
    } else {
        print_help();
    }
}

fn print_help() {
    println!("GuelRust — 구엘룰 필승 끝말잇기 엔진");
    println!();
    println!("사용법:");
    println!("  guel stats [dict.txt]               노드별 W/L/D 통계");
    println!("  guel play  [dict.txt] [시작음]      엔진과 대국 (시작음 예: '가')");
    println!("  guel verify [dict.txt]              필승 포지션 rollout 자가검증");
    println!("  guel help");
    println!();
    println!("dict.txt 는 1줄 1단어 텍스트 파일. 생략 시 내장 샘플 사전 사용.");
}

fn load_dict(path: Option<String>) -> (Dict, Graph) {
    let mut b = DictBuilder::new();
    if let Some(path) = path {
        let result = std::fs::read_to_string(path.clone());
        if let Err(err) = result {
            println!("사전 파일을 열 수 없습니다: {} ({})", path, err);
            std::process::exit(1);
        } else {
            b.feed(&mut LineSource::new(result.unwrap()));
        }
    } else {
        b.feed(&mut LineSource::new(SAMPLE_DICT.to_string()));
    }
    let d = b.build();
    let g = Graph::build(&d);
    (d, g)
}

fn node_name(node: u32) -> String {
    if node < 19 {
        guel_kor::jamo::CHOSEONG[node as usize].to_string()
    } else if node >= 100 {
        let s = node - 100;
        let cho = s / 21;
        let jung = s % 21;
        guel_kor::jamo::compose(cho, jung, 0).unwrap().to_string()
    } else {
        "∅".to_string()
    }
}

fn cmd_stats(d: &Dict, g: &Graph) {
    let t = retrograde(g);
    let mut w = 0;
    let mut l = 0;
    let mut n = 0;
    let mut l0: Vec<u32> = Vec::new();
    for node in 0..NODE_SPACE {
        let lvl = t.get(node);
        if is_win(lvl) {
            w += 1;
        } else if is_loss(lvl) {
            l += 1;
            if lvl == 0 {
                l0.push(node);
            }
        } else {
            n += 1;
        }
    }
    println!("사전 단어: {} 개", d.len());
    println!("W(필승) 노드: {} | L(필패) 노드: {} | D(중립) 노드: {}", w, l, n);
    println!("한방(L0) 노드: {} 개", l0.len());
    let examples = l0.iter().take(10).map(|node| node_name(*node)).collect::<Vec<String>>();
    println!("  예: {}", examples.join(", "));
}

fn cmd_play(d: &Dict, g: &Graph, start_arg: Option<String>) {
    let t = retrograde(g);
    let mut engine = Engine::new(g, &t);

    let start_node = start_arg.map_or_else(
        || {
            for node in 0..NODE_SPACE {
                if is_win(t.get(node)) {
                    return node;
                }
            }
            0
        },
        |s| word_key(s.as_str()).unwrap().key_in,
    );

    println!("시작 소리: {} ({})", node_name(start_node), t.classify(start_node));
    let side = engine.start_side(start_node);
    println!(
        "필승 사이드: {}",
        match side {
            Side::First => "선공 (엔진 먼저)",
            Side::Second => "후공 (사용자 먼저)",
            Side::Either => "어느 쪽이든 필승",
        },
    );

    let mut node = start_node;
    let mut engine_turn = side == Side::First;

    loop {
        println!("현재 소리: {} ({})", node_name(node), engine.classify_str(node));
        if engine_turn {
            let mv = engine.best_move(node);
            if mv.is_none() {
                println!("엔진이 둘 곳이 없습니다. 사용자 승리!");
                return;
            }
            let m = mv.unwrap();
            let text = d.word(m.word_id).text.clone();
            println!("엔진: {}", text);
            node = engine.current_key_out(m.word_id);
            engine.play(m.word_id);
        } else {
            print!("사용자: ");
            let mut line = String::new();
            let r = std::io::stdin().read_line(&mut line);
            if r.is_err() {
                println!("\n게임 종료.");
                return;
            }
            if r.unwrap() == 0 {
                // EOF — 파이프 종료
                println!("\n게임 종료.");
                return;
            }
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // 사전에서 단어 id 찾기 (선형 탐색 — 사전 규모에 비해 저렴)
            let mut wid: Option<u32> = None;
            for w in &d.words {
                if w.text == line {
                    wid = Some(w.id);
                    break;
                }
            }
            if wid.is_none() {
                println!("사전에 없는 단어입니다.");
                continue;
            }
            let wid = wid.unwrap();
            let legal = engine.playable(node);
            if !legal.contains(&wid) {
                println!("이어지는 소리가 맞지 않거나 이미 사용된 단어입니다.");
                continue;
            }
            node = engine.current_key_out(wid);
            engine.play(wid);
        }
        engine_turn = !engine_turn;
    }
}

fn cmd_verify(d: &Dict, g: &Graph) {
    let t = retrograde(g);
    let mut wins = 0;
    let mut losses = 0;
    let mut draws = 0;
    for seed in 0..50u64 {
        for node in 0..NODE_SPACE {
            if !is_win(t.get(node)) {
                continue;
            }
            match simulate_engine_vs_random(g, node, seed) {
                Outcome::EngineWin => wins += 1,
                Outcome::EngineLoss => losses += 1,
                Outcome::Draw => draws += 1,
            }
        }
    }
    println!("사전 단어: {} 개", d.len());
    println!("rollout: 승={} 패={} 무승부={}", wins, losses, draws);
    if losses == 0 {
        println!("검증 통과: 필승 포지션에서 엔진 패배 0회");
    } else {
        println!("검증 실패: 패배 {} 회", losses);
        std::process::exit(1);
    }
}