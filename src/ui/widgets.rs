use egui::{Color32, RichText};

pub fn status_badge(ui: &mut egui::Ui, label: &str, color: Color32) {
    ui.label(RichText::new(label).color(color).strong());
}

pub fn phase_color(phase: &crate::core::model::SessionPhase) -> Color32 {
    use crate::core::model::SessionPhase;
    match phase {
        SessionPhase::Idle => Color32::GRAY,
        SessionPhase::PreLaunch => Color32::YELLOW,
        SessionPhase::Running => Color32::GREEN,
        SessionPhase::Restoring => Color32::from_rgb(255, 165, 0),
        SessionPhase::Completed => Color32::LIGHT_BLUE,
        SessionPhase::Failed => Color32::RED,
        SessionPhase::Interrupted => Color32::from_rgb(255, 100, 100),
    }
}

pub fn level_color(level: &str) -> Color32 {
    match level {
        "INFO" => Color32::WHITE,
        "WARN" => Color32::YELLOW,
        "ERROR" => Color32::RED,
        "SECURITY" => Color32::from_rgb(255, 100, 0),
        "DEBUG" => Color32::GRAY,
        _ => Color32::WHITE,
    }
}
