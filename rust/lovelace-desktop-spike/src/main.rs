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
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app::{SpikeApp, WindowBenchResult};

fn native_options(title: &str) -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title(title),
        ..Default::default()
    }
}

fn main() -> eframe::Result<()> {
    let bench_mode = std::env::args().any(|a| a == "--bench");

    if bench_mode {
        let bench_start = Instant::now();
        let shared: Arc<Mutex<Option<WindowBenchResult>>> = Arc::new(Mutex::new(None));
        let for_app = shared.clone();
        let window_result = match eframe::run_native(
            "Lovelace Desktop Spike (bench)",
            native_options("Lovelace Desktop Spike (bench)"),
            Box::new(move |cc| {
                cc.egui_ctx.set_visuals(egui::Visuals::dark());
                // Watchdog: if the app cannot close itself (a missed repaint, a stuck
                // event loop), force the window closed so `--bench` always terminates.
                let watchdog = cc.egui_ctx.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(15));
                    watchdog.request_repaint();
                    watchdog.send_viewport_cmd(egui::ViewportCommand::Close);
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
        };

        let mut window_result = window_result;
        if window_result.is_none() {
            let mut r = WindowBenchResult::default();
            r.window_error =
                Some("window phase ended without recording startup/idle numbers".to_string());
            window_result = Some(r);
        }
        let bench_total_ms = bench_start.elapsed().as_secs_f64() * 1000.0;
        let line = bench::run_headless(window_result, bench_total_ms);
        println!("SPIKE_BENCH {line}");
        let _ = std::io::stdout().flush();
        // Bench mode is a measurement run, not an interactive session.
        std::process::exit(0);
    }

    eframe::run_native(
        "Lovelace Desktop Spike",
        native_options("Lovelace Desktop Spike - eframe/egui 0.37"),
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(SpikeApp::new(None)))
        }),
    )
}

