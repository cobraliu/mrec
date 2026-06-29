//! Cross-platform mouse & keyboard macro: record + loop replay.
//!
//! Backend: rdev (libX11/XTEST on Linux, native APIs on Windows/macOS).
//! Records mouse moves, left/right/middle clicks (double-click is just two
//! fast clicks, preserved by timing), wheel, and keyboard input. Replays with
//! the original timing, any number of loops.
//!
//! NOTE: works on Windows, macOS and Linux/X11. It does NOT work under a native
//! Wayland session (Wayland blocks global input capture/injection by design).

use rdev::{listen, simulate, Event, EventType, SimulateError};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// One recorded event: seconds since recording start + the rdev event type.
#[derive(Serialize, Deserialize)]
struct Rec {
    t: f64,
    e: EventType,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage:\n  {bin} record <file> [seconds]   (default 30)\n  {bin} replay <file> [loops]     (default 1, 0 = infinite)",
            bin = "mrec"
        );
        std::process::exit(1);
    }

    match args[1].as_str() {
        "record" => {
            let secs: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(30.0);
            record(&args[2], secs);
        }
        "replay" => {
            let loops: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
            replay(&args[2], loops);
        }
        other => {
            eprintln!("unknown mode: {other}");
            std::process::exit(1);
        }
    }
}

/// Live "3 ... 2 ... 1" pre-start countdown on a single line.
fn countdown(prefix: &str, n: u64) {
    for i in (1..=n).rev() {
        print!("\r{prefix} {i} ...   ");
        io::stdout().flush().ok();
        thread::sleep(Duration::from_secs(1));
    }
    print!("\r{:width$}\r", "", width = 40);
    io::stdout().flush().ok();
}

fn record(path: &str, secs: f64) {
    let events: Arc<Mutex<Vec<Rec>>> = Arc::new(Mutex::new(Vec::new()));
    let start = Instant::now();

    // rdev::listen blocks for the whole session, so run it on its own thread.
    // The process exits when recording is done, which tears the thread down.
    let sink = events.clone();
    thread::spawn(move || {
        let cb = move |e: Event| {
            let keep = matches!(
                e.event_type,
                EventType::MouseMove { .. }
                    | EventType::ButtonPress(_)
                    | EventType::ButtonRelease(_)
                    | EventType::KeyPress(_)
                    | EventType::KeyRelease(_)
                    | EventType::Wheel { .. }
            );
            if keep {
                let t = start.elapsed().as_secs_f64();
                sink.lock().unwrap().push(Rec { t, e: e.event_type });
            }
        };
        if let Err(err) = listen(cb) {
            eprintln!("\n[REC] listen error: {err:?}");
            std::process::exit(1);
        }
    });

    countdown("Recording starts in", 3);
    println!("[REC] recording for {secs:.0}s, start now");

    let end = Instant::now() + Duration::from_secs_f64(secs);
    while Instant::now() < end {
        let left = end.saturating_duration_since(Instant::now()).as_secs_f64();
        let n = events.lock().unwrap().len();
        print!("\r[REC] {left:5.1}s left, events={n}   ");
        io::stdout().flush().ok();
        thread::sleep(Duration::from_millis(200));
    }
    print!("\r{:width$}\r", "", width = 50);

    let data = events.lock().unwrap();
    let json = serde_json::to_string(&*data).expect("serialize");
    fs::write(path, json).expect("write file");
    println!("[REC] saved {} events -> {path}", data.len());
}

fn replay(path: &str, loops: u64) {
    let json = fs::read_to_string(path).expect("read file");
    let events: Vec<Rec> = serde_json::from_str(&json).expect("parse json");
    let total = events.last().map(|r| r.t).unwrap_or(0.0);

    countdown("Replay starts in", 3);

    let mut n = 0u64;
    loop {
        n += 1;
        if loops == 0 {
            println!("[REPLAY] loop {n}/inf");
        } else {
            println!("[REPLAY] loop {n}/{loops}");
        }

        let t0 = Instant::now();
        for r in &events {
            // Wait until this event's recorded offset to preserve timing.
            while t0.elapsed().as_secs_f64() < r.t {
                thread::sleep(Duration::from_millis(2));
            }
            match simulate(&r.e) {
                Ok(()) => {}
                Err(SimulateError) => eprintln!("\n[REPLAY] simulate failed for {:?}", r.e),
            }
            // Small settle delay; some OSes drop events fired too fast.
            thread::sleep(Duration::from_millis(1));
            print!("\r[REPLAY] {:5.1}/{total:.1}s   ", t0.elapsed().as_secs_f64());
            io::stdout().flush().ok();
        }
        print!("\r{:width$}\r", "", width = 50);

        if loops != 0 && n >= loops {
            break;
        }
    }
    println!("[REPLAY] done");
}
