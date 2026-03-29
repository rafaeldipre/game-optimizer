use crate::app::AppState;
use crate::persistence::config as config_persistence;
use crate::persistence::session as session_persistence;
use egui::{Color32, RichText};

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Settings");
    ui.separator();

    // Elevation status
    ui.horizontal(|ui| {
        ui.label("Running as Administrator:");
        if state.is_elevated {
            ui.label(RichText::new("YES").color(Color32::GREEN).strong());
        } else {
            ui.label(
                RichText::new("NO — Restart as administrator!")
                    .color(Color32::RED)
                    .strong(),
            );
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("Configuration").strong());

    egui::Grid::new("settings_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Profiles directory:");
            ui.text_edit_singleline(&mut state.config.profiles_dir);
            ui.end_row();

            ui.label("Logs directory:");
            ui.text_edit_singleline(&mut state.config.logs_dir);
            ui.end_row();

            ui.label("Confirm Realtime priority:");
            ui.checkbox(&mut state.config.confirm_realtime_priority, "");
            ui.end_row();

            ui.label("Confirm before launch:");
            ui.checkbox(&mut state.config.confirm_before_launch, "");
            ui.end_row();
        });

    if ui.button("Save settings").clicked() {
        let _ = config_persistence::save_config(&state.config);
        crate::logging::logger::add_to_buffer("INFO", "Settings saved");
    }

    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("Session").strong());

    if ui.button("Clear session.json").clicked() {
        let _ = session_persistence::clear_session();
        *state.shared_session.lock().unwrap() = crate::core::model::SessionState::default();
        crate::logging::logger::add_to_buffer("INFO", "Session file cleared manually");
    }

    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("About").strong());
    ui.label("game-optimizer v0.1.0");
    ui.label("Windows 11 game optimization tool");
    ui.label("Requires administrator privileges");
    ui.add_space(4.0);
    ui.hyperlink_to("GitHub", "https://github.com/");
}
