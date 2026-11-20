use ime_core::{Context, EngineUpdate, InputEngine, Key, KeyEvent, Modifiers};
use omarchy_ime::AppConfig;
use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("--version") | Some("version") => {
            println!("omarchy-ime {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--check") | Some("check") => {
            let path = args.next();
            if let Some(path) = path {
                match omarchy_ime::load_pinyin(path) {
                    Ok(_) => {
                        println!("dictionary: ok");
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("dictionary: {e}");
                        ExitCode::FAILURE
                    }
                }
            } else {
                let user = omarchy_ime::user_dictionary_path();
                let system = omarchy_ime::installed_dictionary_path();
                let selected = if user.is_file() {
                    Some(user)
                } else if system.is_file() {
                    Some(system)
                } else {
                    None
                };
                if let Some(path) = selected {
                    match omarchy_ime::load_pinyin(&path) {
                        Ok(_) => {
                            println!("runtime: ok ({})", path.display());
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("dictionary {}: {e}", path.display());
                            ExitCode::FAILURE
                        }
                    }
                } else {
                    let _ = omarchy_ime::bundled_pinyin();
                    println!("runtime: ok (bundled starter)");
                    ExitCode::SUCCESS
                }
            }
        }
        Some("--bench") | Some("bench") => {
            let input = args.next().unwrap_or_else(|| "nihao".into());
            let iterations = args
                .next()
                .and_then(|n| n.parse::<usize>().ok())
                .unwrap_or(100)
                .clamp(1, 10_000);
            let mut engine = omarchy_ime::load_default_pinyin();
            let ctx = Context::default();
            let events: Vec<_> = input
                .chars()
                .map(|ch| KeyEvent {
                    key: Key::Character(ch),
                    modifiers: Modifiers::default(),
                    repeat: false,
                })
                .collect();
            let mut samples = Vec::with_capacity(iterations.saturating_mul(events.len()));
            for _ in 0..iterations {
                for event in &events {
                    let start = std::time::Instant::now();
                    let _ = engine.handle_key(*event, &ctx);
                    samples.push(start.elapsed());
                }
                engine.reset();
            }
            let total: std::time::Duration = samples.iter().copied().sum();
            let max = samples.iter().copied().max().unwrap_or_default();
            let mean_us = total.as_secs_f64() * 1_000_000.0 / samples.len().max(1) as f64;
            println!(
                "keys={} iterations={} mean={mean_us:.2}us max={}us",
                samples.len(),
                iterations,
                max.as_secs_f64() * 1_000_000.0
            );
            ExitCode::SUCCESS
        }
        Some("--demo") | Some("demo") => {
            let input = args.next().unwrap_or_else(|| "nihao".into());
            let mut engine = omarchy_ime::load_default_pinyin();
            let ctx = Context::default();
            let mut update = EngineUpdate::Noop;
            for ch in input.chars() {
                update = engine.handle_key(
                    KeyEvent {
                        key: Key::Character(ch),
                        modifiers: Modifiers::default(),
                        repeat: false,
                    },
                    &ctx,
                );
            }
            if let EngineUpdate::Candidates(list) = update {
                for (i, c) in list.current_page().iter().enumerate() {
                    println!(
                        "{}\t{}\t{}",
                        i + 1,
                        c.text,
                        c.annotation.as_deref().unwrap_or("")
                    );
                }
            }
            ExitCode::SUCCESS
        }
        _ => {
            let cfg = AppConfig::default();
            println!("omarchy-ime: local Rust engine runtime\nUsage: omarchy-ime --check [DICT.tsv] | --demo [pinyin] | --bench [pinyin] [iterations] | --version\nDefault data: {}", cfg.dictionary_path);
            ExitCode::SUCCESS
        }
    }
}
