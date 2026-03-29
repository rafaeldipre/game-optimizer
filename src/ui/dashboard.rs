use crate::app::{AppState, Tab};
use crate::core::model::SessionPhase;
use crate::ui::widgets::{phase_color, status_badge};
use chrono::Utc;
use egui::{Color32, RichText};

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Dashboard");
    ui.separator();

    // Profile selector
    ui.horizontal(|ui| {
        ui.label("Profile:");
        let profile_names: Vec<String> = state.profiles.iter().map(|p| p.friendly_name.clone()).collect();
        let selected = state.selected_profile_index.unwrap_or(0);
        let mut combo_sel = selected;

        if profile_names.is_empty() {
            ui.label("No profiles found. Create one in the Profiles tab.");
        } else {
            egui::ComboBox::from_id_source("profile_selector")
                .selected_text(profile_names.get(selected).cloned().unwrap_or_default())
                .show_ui(ui, |ui| {
                    for (i, name) in profile_names.iter().enumerate() {
                        ui.selectable_value(&mut combo_sel, i, name);
                    }
                });
            if combo_sel != selected || state.selected_profile_index.is_none() {
                state.selected_profile_index = Some(combo_sel);
            }

            // Quick-access button to edit selected profile
            if ui.button("⚙ Configure").clicked() {
                if let Some(idx) = state.selected_profile_index {
                    state.profile_edit_state = Some(state.profiles[idx].clone());
                    state.is_editing_profile = true;
                    state.active_tab = Tab::Profiles;
                }
            }
        }
    });

    ui.add_space(8.0);

    // Read session state once per frame (lock briefly)
    let (phase, started_at, _profile_name) = {
        let sess = state.shared_session.lock().unwrap();
        (sess.phase.clone(), sess.started_at, sess.profile_name.clone())
    };

    // Session status
    ui.horizontal(|ui| {
        ui.label("Status:");
        let color = phase_color(&phase);
        status_badge(ui, &phase.to_string(), color);
    });

    // Session timer
    if let Some(started) = started_at {
        if phase == SessionPhase::Running {
            let elapsed = Utc::now() - started;
            let secs = elapsed.num_seconds().max(0);
            let h = secs / 3600;
            let m = (secs % 3600) / 60;
            let s = secs % 60;
            ui.label(format!("Duration: {:02}:{:02}:{:02}", h, m, s));
        }
    }

    ui.add_space(8.0);

    let is_running = matches!(phase, SessionPhase::Running | SessionPhase::PreLaunch | SessionPhase::Restoring);

    ui.horizontal(|ui| {
        let can_launch = !is_running && state.selected_profile_index.is_some();
        let launch_btn = ui.add_enabled(can_launch, egui::Button::new("Launch"));

        if launch_btn.clicked() {
            if let Some(idx) = state.selected_profile_index {
                if let Some(profile) = state.profiles.get(idx) {
                    let p = profile.clone();
                    if p.process_priority == crate::core::model::ProcessPriority::Realtime
                        && state.config.confirm_realtime_priority
                    {
                        state.show_realtime_warning = true;
                    } else {
                        launch_profile(state, p);
                    }
                }
            }
        }

        if is_running && ui.button("Stop").clicked() {
            state.stop_session();
        }
    });

    // Realtime warning dialog
    if state.show_realtime_warning {
        egui::Window::new("WARNING: REALTIME Priority")
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label(
                    RichText::new(
                        "REALTIME priority can cause the Windows scheduler to stop\n\
                         servicing hardware interrupts, leading to a system freeze.\n\n\
                         Only use this if you know what you are doing.",
                    )
                    .color(Color32::RED),
                );
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Continue anyway").clicked() {
                        state.show_realtime_warning = false;
                        if let Some(idx) = state.selected_profile_index {
                            if let Some(profile) = state.profiles.get(idx) {
                                let p = profile.clone();
                                launch_profile(state, p);
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        state.show_realtime_warning = false;
                    }
                });
            });
    }

    // Recovery dialog for interrupted sessions
    if state.show_recovery_dialog {
        if let Some(ref interrupted) = state.interrupted_session.clone() {
            let name = interrupted.profile_name.clone().unwrap_or_else(|| "Unknown".to_string());
            let started = interrupted
                .started_at
                .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "Unknown time".to_string());

            egui::Window::new("Interrupted Session Detected")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(format!(
                        "An incomplete session for profile '{}' was found\nstarted at {}.\n\n\
                         Do you want to restore the system to its previous state?",
                        name, started
                    ));
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Restore now").clicked() {
                            state.show_recovery_dialog = false;
                            restore_interrupted_session(state);
                        }
                        if ui.button("Ignore").clicked() {
                            state.show_recovery_dialog = false;
                            let _ = crate::persistence::session::clear_session();
                            state.interrupted_session = None;
                            crate::logging::logger::add_to_buffer("WARN", "Recovery: user chose to ignore interrupted session");
                        }
                    });
                });
        }
    }

    ui.add_space(12.0);
    ui.separator();
    ui.label(RichText::new("Recent Log").strong());
    ui.add_space(4.0);

    let entries = crate::logging::logger::get_log_entries();
    let last_entries: Vec<_> = entries.iter().rev().take(10).collect::<Vec<_>>().into_iter().rev().collect();

    egui::ScrollArea::vertical()
        .max_height(150.0)
        .show(ui, |ui| {
            for entry in &last_entries {
                let color = crate::ui::widgets::level_color(&entry.level);
                ui.label(
                    RichText::new(format!("[{}] [{}] {}", entry.timestamp, entry.level, entry.message))
                        .color(color)
                        .monospace()
                        .size(11.0),
                );
            }
        });
}

fn launch_profile(state: &mut AppState, profile: crate::core::model::Profile) {
    state.reset_stop_signal();

    // Initialize shared session for the new run
    {
        let mut sess = state.shared_session.lock().unwrap();
        *sess = crate::core::model::SessionState::default();
        sess.phase = crate::core::model::SessionPhase::PreLaunch;
        sess.profile_name = Some(profile.friendly_name.clone());
        sess.started_at = Some(Utc::now());
    }

    let signal = state.stop_signal.clone();
    let shared_session = state.shared_session.clone();

    tokio::spawn(async move {
        match crate::core::optimizer::Optimizer::launch_profile(&profile, shared_session, signal).await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("[ERROR] Optimizer error: {}", e);
                crate::logging::logger::add_to_buffer("ERROR", &format!("Optimizer error: {}", e));
            }
        }
    });
}

fn restore_interrupted_session(state: &mut AppState) {
    if let Some(interrupted) = state.interrupted_session.take() {
        let snapshot = interrupted.pre_session_snapshot.clone();
        let profile_name = interrupted.profile_name.clone().unwrap_or_default();

        // Build a minimal fake profile just for restore (only exe path needed for GPU restore)
        let mut fake_profile = crate::core::model::Profile::default();
        fake_profile.friendly_name = profile_name.clone();

        // Try to find the real profile
        if let Some(p) = state.profiles.iter().find(|p| p.friendly_name == profile_name) {
            fake_profile = p.clone();
        }

        let shared_session = state.shared_session.clone();

        tokio::spawn(async move {
            if let Some(snap) = snapshot {
                match crate::core::optimizer::Optimizer::restore_system(&snap, &fake_profile).await {
                    Ok(_) => {
                        crate::logging::logger::add_to_buffer("INFO", "Recovery restore completed");
                    }
                    Err(e) => {
                        crate::logging::logger::add_to_buffer("ERROR", &format!("Recovery restore failed: {}", e));
                    }
                }
            }
            let _ = crate::persistence::session::clear_session();
            shared_session.lock().unwrap().phase = crate::core::model::SessionPhase::Idle;
        });
    }
}
