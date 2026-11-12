//! Lovelace Desktop v4 - native immediate-mode spike (eframe/egui 0.37).
//!
//! Run without arguments for an inspectable window; run with `--bench` to measure
//! S1-S7, print one `SPIKE_BENCH {json}` line and exit.

mod app;
mod bench;
mod cpu;
mod editor;
mod json;
mod plot;
mod sample;
mod syntax;

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app::{SpikeApp, WindowBenchResult};

fn native_options(title: &str, position: Option<[f32; 2]>) -> eframe::NativeOptions {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([900.0, 600.0])
        .with_title(title);
    if let Some(p) = position {
        viewport = viewport.with_position(p);
    }
    eframe::NativeOptions {
        viewport,
        ..Default::default()
    }
}

fn main() -> eframe::Result<()> {
    let bench_mode = std::env::args().any(|a| a == "--bench");

    if bench_mode {
        let bench_start = Instant::now();

        // Headless S1-S6 are computed BEFORE the window opens. That way a watchdog can
        // always print a complete SPIKE_BENCH line even if the windowed phase never
        // returns (the self-close path is flaky in this environment).
        let headless = bench::run_headless();
        let printed = Arc::new(AtomicBool::new(false));
        let simulate = std::env::var("SPIKE_BENCH_SIMULATE").unwrap_or_default();
        let deadline = if simulate == "hang" {
            Duration::from_secs(2) // test hook: reach the watchdog quickly
        } else {
            Duration::from_secs(20)
        };

        {
            let headless = headless.clone();
            let printed = printed.clone();
            std::thread::spawn(move || {
                std::thread::sleep(deadline);
                if !printed.swap(true, Ordering::SeqCst) {
                    let total = bench_start.elapsed().as_secs_f64() * 1000.0;
                    let line = bench::window_timeout_line(
                        &headless,
                        total,
                        "window did not close before the watchdog deadline; the window-close path is flaky",
                    );
                    println!("SPIKE_BENCH {line}");
                    let _ = std::io::stdout().flush();
                    std::process::exit(0);
                }
            });
        }

        if simulate == "hang" {
            // Test hook: behave like a stuck event loop; the watchdog must print and exit.
            std::thread::sleep(Duration::from_secs(10));
        }

        let shared: Arc<Mutex<Option<WindowBenchResult>>> = Arc::new(Mutex::new(None));
        let for_app = shared.clone();
        let window_result = if simulate == "nowindow" {
            None
        } else {
            match eframe::run_native(
                "Lovelace Desktop Spike (bench)",
                native_options("Lovelace Desktop Spike (bench)", Some([16.0, 16.0])),
                Box::new(move |cc| {
                    cc.egui_ctx.set_visuals(egui::Visuals::dark());
                    // First-line defence: ask the viewport to close after 15 s in case the
                    // app's own measured close is missed. The print watchdog above is the
                    // guarantee that a JSON line is produced regardless.
                    let closer = cc.egui_ctx.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_secs(15));
                        closer.request_repaint();
                        closer.send_viewport_cmd(egui::ViewportCommand::Close);
                    });
                    Ok(Box::new(SpikeApp::new(Some(for_app))))
                }),
            ) {
                Ok(()) => shared.lock().unwrap().take(),
                Err(e) => {
                    let mut r = WindowBenchResult::default();
                    r.window_error = Some(format!("eframe::run_native failed: {e}"));
                    Some(r)
                }
            }
        };

        if printed.swap(true, Ordering::SeqCst) {
            // The watchdog already printed a line and exited; this is unreachable in
            // practice, but keeps the single-print invariant explicit.
            std::process::exit(0);
        }

        let mut window_result = window_result;
        if window_result.is_none() {
            let mut r = WindowBenchResult::default();
            r.window_error =
                Some("window phase ended without recording startup/idle numbers".to_string());
            window_result = Some(r);
        }
        let bench_total_ms = bench_start.elapsed().as_secs_f64() * 1000.0;
        let line = bench::finalize(headless, window_result, bench_total_ms);
        println!("SPIKE_BENCH {line}");
        let _ = std::io::stdout().flush();
        // Bench mode is a measurement run, not an interactive session.
        std::process::exit(0);
    }

    eframe::run_native(
        "Lovelace Desktop Spike",
        native_options("Lovelace Desktop Spike - eframe/egui 0.37", None),
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(SpikeApp::new(None)))
        }),
    )
}
