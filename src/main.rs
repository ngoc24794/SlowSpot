#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use eframe::egui;
use slowspot::collector::{MockSource, SampleSource, SysinfoSource};
use slowspot::config::AppConfig;
use slowspot::platform::dialog;
use slowspot::ui::strings;
use slowspot::ui::{ActionRunner, DemoActions, RealActions, SlowSpotApp};
use slowspot::APP_NAME;

/// Command-line configuration options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Whether to run in simulated demonstration mode without affecting OS state.
    pub demo: bool,
    /// Optional output directory for deterministic screenshots.
    pub screenshot_dir: Option<PathBuf>,
    /// Target viewport size (width, height).
    pub size: (u32, u32),
    /// Whether to run layout assertions and exit.
    pub assert_layout: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            demo: false,
            screenshot_dir: None,
            size: (1000, 680),
            assert_layout: false,
        }
    }
}

/// Parses command-line arguments manually without third-party crates.
pub fn parse_args(args: impl Iterator<Item = String>) -> Options {
    let mut options = Options::default();
    let mut iter = args.peekable();

    while let Some(arg) = iter.next() {
        if arg == "--demo" {
            options.demo = true;
        } else if arg == "--assert-layout" {
            options.assert_layout = true;
        } else if arg == "--screenshot" {
            if let Some(dir) = iter.next() {
                options.screenshot_dir = Some(PathBuf::from(dir));
            }
        } else if let Some(val) = arg.strip_prefix("--screenshot=") {
            options.screenshot_dir = Some(PathBuf::from(val));
        } else if arg == "--size" {
            if let Some(dim) = iter.next() {
                if let Some(parsed) = parse_dimension(&dim) {
                    options.size = parsed;
                }
            }
        } else if let Some(val) = arg.strip_prefix("--size=") {
            if let Some(parsed) = parse_dimension(val) {
                options.size = parsed;
            }
        }
    }
    options
}

fn parse_dimension(s: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = s.split(['x', 'X']).collect();
    if parts.len() == 2 {
        if let (Ok(w), Ok(h)) = (
            parts[0].trim().parse::<u32>(),
            parts[1].trim().parse::<u32>(),
        ) {
            return Some((w, h));
        }
    }
    None
}

/// Root application wrapper holding the active UI dashboard and the background collector handle.
struct SlowSpotMainApp {
    app: SlowSpotApp,
    _collector: slowspot::collector::CollectorHandle,
}

impl eframe::App for SlowSpotMainApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui(ui, frame);
    }
}

/// Deterministic demo runner capturing screenshots across fixed lifecycle phases.
struct ScreenshotApp {
    app: SlowSpotApp,
    outdir: PathBuf,
    width: u32,
    height: u32,
    mock: MockSource,
    analyzer: slowspot::analyzer::Analyzer,
    current_phase_idx: usize,
    phase_frame_count: usize,
    waiting_for_save: Arc<AtomicBool>,
    recorded_layouts: Vec<(&'static str, slowspot::ui::layout_assert::LayoutSnapshot)>,
}

const PHASES: [(&str, u64); 4] = [
    ("calm", 4),
    ("suspicious", 15),
    ("culprit", 30),
    ("cooling", 45),
];

impl eframe::App for ScreenshotApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.current_phase_idx >= PHASES.len() {
            // All phases captured. Verify cross-phase stability before exit.
            let phase_refs: Vec<(&str, &slowspot::ui::layout_assert::LayoutSnapshot)> =
                self.recorded_layouts.iter().map(|(n, l)| (*n, l)).collect();
            if let Err(err) = slowspot::ui::layout_assert::assert_layout_stability(&phase_refs) {
                eprintln!("Screenshot layout stability assertion failed: {err}");
                std::process::exit(1);
            }
            std::process::exit(0);
        }

        let (phase_name, target_tick) = PHASES[self.current_phase_idx];

        if self.phase_frame_count == 0 {
            // Advance simulation deterministically up to the target phase tick
            while self.mock.tick_count() <= target_tick {
                let raw = self.mock.sample();
                let snap = self.analyzer.analyze(raw);
                self.app.push_snapshot(snap, std::time::Instant::now());
            }
        }

        // Render dashboard frame
        self.app.render_ui(ui, std::time::Instant::now());

        if let Some(layout) = &self.app.last_layout {
            let min_rows = if self.width >= 1000 { 8 } else { 4 };
            if let Err(err) = layout.assert_valid(phase_name, min_rows) {
                eprintln!("Layout validation failure in phase '{phase_name}': {err}");
                std::process::exit(1);
            }
        }

        if self.phase_frame_count == 1 {
            let filename = format!("{}_{}x{}.bmp", phase_name, self.width, self.height);
            let out_path = self.outdir.join(filename);
            let waiting = Arc::clone(&self.waiting_for_save);
            waiting.store(true, Ordering::SeqCst);

            if let Some(layout) = self.app.last_layout.clone() {
                self.recorded_layouts.push((phase_name, layout));
            }

            let is_last_phase = self.current_phase_idx + 1 >= PHASES.len();
            if is_last_phase {
                let phase_refs: Vec<(&str, &slowspot::ui::layout_assert::LayoutSnapshot)> =
                    self.recorded_layouts.iter().map(|(n, l)| (*n, l)).collect();
                if let Err(err) = slowspot::ui::layout_assert::assert_layout_stability(&phase_refs)
                {
                    eprintln!("Screenshot layout stability assertion failed: {err}");
                    std::process::exit(1);
                }
            }

            let ctx = ui.ctx().clone();
            ui.ctx().request_screenshot(move |image| {
                if let Err(err) = slowspot::ui::bmp::save_bmp(&out_path, &image) {
                    eprintln!("Failed to save screenshot {}: {err}", out_path.display());
                    std::process::exit(1);
                }
                println!("Saved screenshot: {}", out_path.display());
                waiting.store(false, Ordering::SeqCst);
                if is_last_phase {
                    std::process::exit(0);
                }
                ctx.request_repaint();
            });
            ui.ctx().request_repaint();
            self.phase_frame_count += 1;
        } else if self.phase_frame_count >= 2 {
            if !self.waiting_for_save.load(Ordering::SeqCst) {
                self.current_phase_idx += 1;
                self.phase_frame_count = 0;
            }
            ui.ctx().request_repaint();
        } else {
            self.phase_frame_count += 1;
            ui.ctx().request_repaint();
        }
    }
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("SlowSpot fatal panic: {panic_info}");
        dialog::show_error_box(APP_NAME, strings::ERR_UNEXPECTED_PANIC);
        std::process::exit(1);
    }));
}

fn main() {
    install_panic_hook();

    let options = parse_args(std::env::args().skip(1));

    if options.assert_layout {
        println!("Running machine-checkable layout assertions...");
        let sizes = if options.size != (1000, 680) {
            vec![options.size]
        } else {
            vec![(1000, 680), (760, 520)]
        };

        for (w, h) in sizes {
            let min_rows = if w >= 1000 { 8 } else { 4 };
            if let Err(err) =
                slowspot::ui::layout_assert::verify_layout_for_size(w as f32, h as f32, min_rows)
            {
                eprintln!("Layout assertion failure at {w}x{h}: {err}");
                std::process::exit(1);
            }
            println!(
                "Layout assertions passed at {w}x{h} (>= {min_rows} rows, all phases stable)."
            );
        }
        println!("All layout assertions passed successfully.");
        std::process::exit(0);
    }

    let (win_w, win_h) = (options.size.0 as f32, options.size.1 as f32);
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([win_w, win_h])
            .with_min_inner_size([win_w.min(760.0), win_h.min(520.0)])
            .with_title(APP_NAME),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    let app_creator: eframe::AppCreator = Box::new(move |cc| {
        if let Some(screenshot_dir) = options.screenshot_dir {
            let (_, dummy_rx) = mpsc::channel();
            let app = SlowSpotApp::new(cc, dummy_rx, Box::new(DemoActions)).with_demo(true);

            return Ok(Box::new(ScreenshotApp {
                app,
                outdir: screenshot_dir,
                width: options.size.0,
                height: options.size.1,
                mock: MockSource::demo(),
                analyzer: slowspot::analyzer::Analyzer::default(),
                current_phase_idx: 0,
                phase_frame_count: 0,
                waiting_for_save: Arc::new(AtomicBool::new(false)),
                recorded_layouts: Vec::new(),
            }));
        }

        let ctx = cc.egui_ctx.clone();
        let on_snapshot = Box::new(move || {
            ctx.request_repaint();
        });

        let (source, actions): (Box<dyn SampleSource>, Box<dyn ActionRunner>) = if options.demo {
            (Box::new(MockSource::demo()), Box::new(DemoActions))
        } else {
            (Box::new(SysinfoSource::new()), Box::new(RealActions))
        };

        let mut cfg = AppConfig::default();
        if options.demo {
            cfg.watch_retention_secs = 20;
        }
        let mut handle = slowspot::collector::spawn(source, cfg, on_snapshot);

        let (_, dummy_rx) = mpsc::channel();
        let rx = std::mem::replace(&mut handle.rx, dummy_rx);

        let app = SlowSpotApp::new(cc, rx, actions).with_demo(options.demo);

        Ok(Box::new(SlowSpotMainApp {
            app,
            _collector: handle,
        }))
    });

    if let Err(err) = eframe::run_native(APP_NAME, native_options, app_creator) {
        eprintln!("Failed to launch {APP_NAME}: {err}");
        dialog::show_error_box(APP_NAME, strings::ERR_OPENGL_INIT);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_empty() {
        let opts = parse_args(std::iter::empty());
        assert_eq!(opts, Options::default());
    }

    #[test]
    fn test_parse_args_demo() {
        let args = vec!["--demo".to_string()];
        let opts = parse_args(args.into_iter());
        assert_eq!(
            opts,
            Options {
                demo: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_parse_args_screenshot_and_size() {
        let args = vec![
            "--demo".to_string(),
            "--screenshot".to_string(),
            "target/shots".to_string(),
            "--size".to_string(),
            "760x520".to_string(),
        ];
        let opts = parse_args(args.into_iter());
        assert_eq!(
            opts,
            Options {
                demo: true,
                screenshot_dir: Some(PathBuf::from("target/shots")),
                size: (760, 520),
                assert_layout: false,
            }
        );
    }

    #[test]
    fn test_parse_args_screenshot_and_size_equals_format() {
        let args = vec![
            "--demo".to_string(),
            "--screenshot=target/shots".to_string(),
            "--size=1000x680".to_string(),
            "--assert-layout".to_string(),
        ];
        let opts = parse_args(args.into_iter());
        assert_eq!(
            opts,
            Options {
                demo: true,
                screenshot_dir: Some(PathBuf::from("target/shots")),
                size: (1000, 680),
                assert_layout: true,
            }
        );
    }

    #[test]
    fn test_parse_args_unknown_and_help() {
        let args = vec![
            "--help".to_string(),
            "-h".to_string(),
            "--unknown".to_string(),
            "-v".to_string(),
            "positional".to_string(),
        ];
        let opts = parse_args(args.into_iter());
        assert_eq!(opts, Options::default());
    }
}
