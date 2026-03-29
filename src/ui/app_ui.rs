use crate::app::{AppState, Tab};
use crate::ui::logs_tab::LogFilter;
use eframe::egui;

pub struct GameOptimizerApp {
    pub state: AppState,
    pub log_filter: LogFilter,
}

impl GameOptimizerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        GameOptimizerApp {
            state: AppState::new(),
            log_filter: LogFilter::default(),
        }
    }
}

impl eframe::App for GameOptimizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Only poll every second when a session is active (to update the timer).
        // When idle, let eframe sleep until the next user event.
        let phase = self.state.shared_session.lock().unwrap().phase.clone();
        let is_active = matches!(
            phase,
            crate::core::model::SessionPhase::PreLaunch
                | crate::core::model::SessionPhase::Running
                | crate::core::model::SessionPhase::Restoring
        );
        if is_active {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }

        egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.state.active_tab == Tab::Dashboard, "Dashboard")
                    .clicked()
                {
                    self.state.active_tab = Tab::Dashboard;
                }
                if ui
                    .selectable_label(self.state.active_tab == Tab::Profiles, "Profiles")
                    .clicked()
                {
                    self.state.active_tab = Tab::Profiles;
                }
                if ui
                    .selectable_label(self.state.active_tab == Tab::Logs, "Logs")
                    .clicked()
                {
                    self.state.active_tab = Tab::Logs;
                }
                if ui
                    .selectable_label(self.state.active_tab == Tab::Settings, "Settings")
                    .clicked()
                {
                    self.state.active_tab = Tab::Settings;
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state.active_tab {
                Tab::Dashboard => crate::ui::dashboard::render(ui, &mut self.state),
                Tab::Profiles => crate::ui::profiles_tab::render(ui, &mut self.state),
                Tab::Logs => {
                    crate::ui::logs_tab::render(ui, &mut self.state, &mut self.log_filter)
                }
                Tab::Settings => crate::ui::settings_tab::render(ui, &mut self.state),
            }
        });
    }
}
