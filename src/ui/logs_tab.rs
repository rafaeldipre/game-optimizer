use crate::app::AppState;
use egui::RichText;

pub struct LogFilter {
    pub show_info: bool,
    pub show_warn: bool,
    pub show_error: bool,
    pub show_security: bool,
    pub show_debug: bool,
    pub auto_scroll: bool,
}

impl Default for LogFilter {
    fn default() -> Self {
        LogFilter {
            show_info: true,
            show_warn: true,
            show_error: true,
            show_security: true,
            show_debug: false,
            auto_scroll: true,
        }
    }
}

pub fn render(ui: &mut egui::Ui, _state: &mut AppState, filter: &mut LogFilter) {
    ui.heading("Logs");
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.checkbox(&mut filter.show_info, "INFO");
        ui.checkbox(&mut filter.show_warn, "WARN");
        ui.checkbox(&mut filter.show_error, "ERROR");
        ui.checkbox(&mut filter.show_security, "SECURITY");
        ui.checkbox(&mut filter.show_debug, "DEBUG");
        ui.separator();
        ui.checkbox(&mut filter.auto_scroll, "Auto-scroll");

        if ui.button("Clear viewer").clicked() {
            crate::logging::logger::clear_buffer();
        }

        if ui.button("Open logs folder").clicked() {
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("explorer.exe")
                    .arg("logs")
                    .spawn();
            }
        }
    });

    ui.separator();

    let entries = crate::logging::logger::get_log_entries();
    let filtered: Vec<_> = entries
        .iter()
        .filter(|e| match e.level.as_str() {
            "INFO" => filter.show_info,
            "WARN" => filter.show_warn,
            "ERROR" => filter.show_error,
            "SECURITY" => filter.show_security,
            "DEBUG" => filter.show_debug,
            _ => true,
        })
        .collect();

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .stick_to_bottom(filter.auto_scroll)
        .show(ui, |ui| {
            for entry in &filtered {
                let color = crate::ui::widgets::level_color(&entry.level);
                ui.label(
                    RichText::new(format!(
                        "[{}] [{}] {}",
                        entry.timestamp, entry.level, entry.message
                    ))
                    .color(color)
                    .monospace()
                    .size(11.0),
                );
            }
        });
}
