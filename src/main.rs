#![windows_subsystem = "windows"]

mod app;
mod core;
mod logging;
mod persistence;
mod security;
mod ui;

#[cfg(target_os = "windows")]
mod windows;

use crate::ui::app_ui::GameOptimizerApp;

#[tokio::main]
async fn main() {
    // Initialise logging first
    let config = persistence::config::load_config();
    let _ = logging::logger::init_logging(&config.logs_dir);

    #[cfg(target_os = "windows")]
    {
        if !windows::elevation::is_elevated() {
            // Show a native error dialog and exit
            unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    MessageBoxW, MB_ICONERROR, MB_OK,
                };
                let msg: Vec<u16> = "Game Optimizer requires administrator privileges.\n\n\
                                      Please right-click and select 'Run as administrator'."
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let title: Vec<u16> = "Elevation Required"
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();

                MessageBoxW(0, msg.as_ptr(), title.as_ptr(), MB_ICONERROR | MB_OK);
            }
            std::process::exit(1);
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Game Optimizer")
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([700.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Game Optimizer",
        native_options,
        Box::new(|cc| Box::new(GameOptimizerApp::new(cc))),
    )
    .unwrap();
}
