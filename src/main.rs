#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::mpsc;

use eframe::egui;
use slowspot::collector::{MockSource, SampleSource, SysinfoSource};
use slowspot::config::AppConfig;
use slowspot::platform::dialog;
use slowspot::ui::strings;
use slowspot::ui::{ActionRunner, DemoActions, RealActions, SlowSpotApp};
use slowspot::APP_NAME;

/// Command-line configuration options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Options {
    /// Whether to run in simulated demonstration mode without affecting OS state.
    pub demo: bool,
}

/// Parses command-line arguments manually without third-party crates.
///
/// Unknown arguments and help flags (`--help`, `-h`) are ignored cleanly without crashing.
pub fn parse_args(args: impl Iterator<Item = String>) -> Options {
    let mut options = Options::default();
    for arg in args {
        if arg == "--demo" {
            options.demo = true;
        }
    }
    options
}

/// Root application wrapper holding the active UI dashboard and the background collector handle.
///
/// Ensures the background metrics thread is orderly terminated and joined upon window closure.
struct SlowSpotMainApp {
    app: SlowSpotApp,
    _collector: slowspot::collector::CollectorHandle,
}

impl eframe::App for SlowSpotMainApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui(ui, frame);
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

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 680.0])
            .with_min_inner_size([760.0, 520.0])
            .with_title(APP_NAME),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    let app_creator: eframe::AppCreator = Box::new(move |cc| {
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
            // Accelerated watch retention for demo scenario lifecycle (Overloaded -> Cooling -> Recovered <= 60s)
            cfg.watch_retention_secs = 20;
        }
        let mut handle = slowspot::collector::spawn(source, cfg, on_snapshot);

        // Safely extract the receiving channel without prematurely stopping the collector
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
        assert_eq!(opts, Options { demo: false });
    }

    #[test]
    fn test_parse_args_demo() {
        let args = vec!["--demo".to_string()];
        let opts = parse_args(args.into_iter());
        assert_eq!(opts, Options { demo: true });
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
        assert_eq!(opts, Options { demo: false });
    }

    #[test]
    fn test_parse_args_demo_with_mixed_flags() {
        let args = vec![
            "--some-flag".to_string(),
            "--demo".to_string(),
            "--another".to_string(),
        ];
        let opts = parse_args(args.into_iter());
        assert_eq!(opts, Options { demo: true });
    }
}
