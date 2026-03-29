use crate::app::AppState;
use crate::core::model::{
    GpuPreference, ProcessAction, ProcessPriority, Profile, ServiceAction,
};
use crate::persistence::profiles as profiles_persistence;
use chrono::Utc;
use egui::RichText;
use uuid::Uuid;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Profiles");
    ui.separator();

    // Left panel: profile list + controls
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_min_width(200.0);
            ui.label(RichText::new("Profile List").strong());

            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    let count = state.profiles.len();
                    for i in 0..count {
                        let selected = state.selected_profile_index == Some(i);
                        let name = state.profiles[i].friendly_name.clone();
                        if ui.selectable_label(selected, &name).clicked() {
                            state.selected_profile_index = Some(i);
                        }
                    }
                });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("New").clicked() {
                    state.profile_edit_state = Some(Profile::default());
                    state.is_editing_profile = true;
                }

                let has_sel = state.selected_profile_index.is_some();

                if ui.add_enabled(has_sel, egui::Button::new("Edit")).clicked() {
                    if let Some(idx) = state.selected_profile_index {
                        state.profile_edit_state = Some(state.profiles[idx].clone());
                        state.is_editing_profile = true;
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("Duplicate"))
                    .clicked()
                {
                    if let Some(idx) = state.selected_profile_index {
                        let mut dup = state.profiles[idx].clone();
                        dup.id = Uuid::new_v4();
                        dup.name = format!("{}-copy", dup.name);
                        dup.friendly_name = format!("{} (Copy)", dup.friendly_name);
                        dup.created_at = Utc::now();
                        dup.updated_at = Utc::now();
                        let _ = profiles_persistence::save_profile(&dup);
                        state.reload_profiles();
                    }
                }
            });

            ui.horizontal(|ui| {
                let has_sel = state.selected_profile_index.is_some();

                if ui
                    .add_enabled(has_sel, egui::Button::new("Delete"))
                    .clicked()
                {
                    if let Some(idx) = state.selected_profile_index {
                        let profile = state.profiles[idx].clone();
                        let _ = profiles_persistence::delete_profile(&profile);
                        state.selected_profile_index = None;
                        state.reload_profiles();
                    }
                }

                if ui
                    .add_enabled(has_sel, egui::Button::new("Export JSON"))
                    .clicked()
                {
                    if let Some(idx) = state.selected_profile_index {
                        let profile = state.profiles[idx].clone();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .save_file()
                        {
                            let _ = profiles_persistence::export_profile(&profile, &path);
                        }
                    }
                }

                if ui.button("Import JSON").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .pick_file()
                    {
                        match profiles_persistence::import_profile(&path) {
                            Ok(profile) => {
                                let _ = profiles_persistence::save_profile(&profile);
                                state.reload_profiles();
                            }
                            Err(e) => {
                                tracing::warn!("Import failed: {}", e);
                            }
                        }
                    }
                }
            });
        });

        ui.separator();

        // Right panel: editor or read-only view
        if state.is_editing_profile {
            let mut should_save = false;
            let mut should_cancel = false;

            if let Some(ref mut edit) = state.profile_edit_state {
                // Temporarily extract the input strings to avoid double-borrow
                let mut new_svc = std::mem::take(&mut state.new_service_input);
                let mut new_proc = std::mem::take(&mut state.new_process_input);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_profile_editor(ui, edit, &mut new_svc, &mut new_proc);

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            should_save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            should_cancel = true;
                        }
                    });
                });

                // Put the strings back
                state.new_service_input = new_svc;
                state.new_process_input = new_proc;
            }

            if should_save {
                if let Some(ref mut p) = state.profile_edit_state {
                    p.updated_at = Utc::now();
                    let _ = profiles_persistence::save_profile(p);
                }
                state.reload_profiles();
                state.is_editing_profile = false;
                state.profile_edit_state = None;
                state.new_service_input.clear();
                state.new_process_input.clear();
            }
            if should_cancel {
                state.is_editing_profile = false;
                state.profile_edit_state = None;
                state.new_service_input.clear();
                state.new_process_input.clear();
            }
        } else if let Some(idx) = state.selected_profile_index {
            if let Some(profile) = state.profiles.get(idx) {
                ui.vertical(|ui| {
                    ui.label(RichText::new(&profile.friendly_name).heading());
                    ui.label(format!("ID: {}", profile.id));
                    ui.label(format!("Executable: {}", profile.executable_path));
                    ui.label(format!("Priority: {}", profile.process_priority));
                    ui.label(format!("GPU: {}", profile.gpu_preference));
                    ui.label(format!("Services: {}", profile.services.len()));
                    ui.label(format!("Processes: {}", profile.processes.len()));
                    if !profile.notes.is_empty() {
                        ui.separator();
                        ui.label(RichText::new("Notes:").strong());
                        ui.label(&profile.notes);
                    }
                });
            }
        } else {
            ui.label("Select a profile or create a new one.");
        }
    });
}

fn render_profile_editor(
    ui: &mut egui::Ui,
    profile: &mut Profile,
    new_service_input: &mut String,
    new_process_input: &mut String,
) {
    ui.label(RichText::new("Edit Profile").strong());
    ui.separator();

    egui::Grid::new("profile_fields")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Name (id):");
            ui.text_edit_singleline(&mut profile.name);
            ui.end_row();

            ui.label("Friendly Name:");
            ui.text_edit_singleline(&mut profile.friendly_name);
            ui.end_row();

            ui.label("Executable:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut profile.executable_path);
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Executable", &["exe"])
                        .pick_file()
                    {
                        profile.executable_path = path.to_string_lossy().to_string();
                        // Auto-fill name fields from filename if not already set
                        if profile.name.is_empty() {
                            if let Some(stem) = path.file_stem() {
                                profile.name =
                                    stem.to_string_lossy().to_lowercase().replace(' ', "-");
                                profile.friendly_name = stem.to_string_lossy().to_string();
                            }
                        }
                    }
                }
            });
            ui.end_row();

            ui.label("Launch Args:");
            let mut args = profile.launch_args.clone().unwrap_or_default();
            ui.text_edit_singleline(&mut args);
            profile.launch_args = if args.is_empty() { None } else { Some(args) };
            ui.end_row();

            ui.label("Working Directory:");
            let mut workdir = profile.working_directory.clone().unwrap_or_default();
            ui.text_edit_singleline(&mut workdir);
            profile.working_directory = if workdir.is_empty() { None } else { Some(workdir) };
            ui.end_row();

            ui.label("Process Priority:");
            egui::ComboBox::from_id_source("priority_combo")
                .selected_text(profile.process_priority.to_string())
                .show_ui(ui, |ui| {
                    let priorities = [
                        ProcessPriority::Idle,
                        ProcessPriority::BelowNormal,
                        ProcessPriority::Normal,
                        ProcessPriority::AboveNormal,
                        ProcessPriority::High,
                        ProcessPriority::Realtime,
                    ];
                    for p in &priorities {
                        ui.selectable_value(
                            &mut profile.process_priority,
                            p.clone(),
                            p.to_string(),
                        );
                    }
                });
            ui.end_row();

            if profile.process_priority == ProcessPriority::Realtime {
                ui.label("");
                ui.label(
                    egui::RichText::new("WARNING: REALTIME can freeze the system!")
                        .color(egui::Color32::RED),
                );
                ui.end_row();
            }

            ui.label("GPU Preference:");
            egui::ComboBox::from_id_source("gpu_combo")
                .selected_text(profile.gpu_preference.to_string())
                .show_ui(ui, |ui| {
                    for g in &[
                        GpuPreference::Default,
                        GpuPreference::PowerSaving,
                        GpuPreference::HighPerformance,
                    ] {
                        ui.selectable_value(
                            &mut profile.gpu_preference,
                            g.clone(),
                            g.to_string(),
                        );
                    }
                });
            ui.end_row();

            ui.label("Trim background:");
            ui.checkbox(&mut profile.memory_optimization.trim_other_processes, "");
            ui.end_row();

            ui.label("Delays (ms):");
            ui.horizontal(|ui| {
                ui.label("Svc:");
                ui.add(
                    egui::DragValue::new(&mut profile.delays.after_services_ms)
                        .clamp_range(0u64..=30000u64),
                );
                ui.label("Proc:");
                ui.add(
                    egui::DragValue::new(&mut profile.delays.after_processes_ms)
                        .clamp_range(0u64..=30000u64),
                );
                ui.label("Launch:");
                ui.add(
                    egui::DragValue::new(&mut profile.delays.after_launch_ms)
                        .clamp_range(0u64..=30000u64),
                );
            });
            ui.end_row();

            ui.label("Notes:");
            ui.text_edit_multiline(&mut profile.notes);
            ui.end_row();
        });

    // Services section
    ui.add_space(8.0);
    ui.label(RichText::new("Services to Stop").strong());

    let mut remove_svc_idx: Option<usize> = None;
    for (i, svc) in profile.services.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.checkbox(&mut svc.stop_on_launch, "Stop");
            ui.label(&svc.service_name);
            if !svc.display_name.is_empty() && svc.display_name != svc.service_name {
                ui.label(format!("({})", svc.display_name));
            }
            if ui.small_button("x").clicked() {
                remove_svc_idx = Some(i);
            }
        });
    }
    if let Some(i) = remove_svc_idx {
        profile.services.remove(i);
    }

    ui.horizontal(|ui| {
        ui.text_edit_singleline(new_service_input);
        if ui.button("Add Service").clicked() && !new_service_input.is_empty() {
            let name = new_service_input.clone();
            profile.services.push(ServiceAction {
                service_name: name.clone(),
                display_name: name,
                stop_on_launch: true,
                was_running: None,
            });
            new_service_input.clear();
        }
    });

    // Processes section
    ui.add_space(8.0);
    ui.label(RichText::new("Processes to Kill").strong());

    let mut remove_proc_idx: Option<usize> = None;
    for (i, proc) in profile.processes.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.checkbox(&mut proc.kill_on_launch, "Kill");
            ui.checkbox(&mut proc.relaunch_after, "Relaunch");
            ui.checkbox(&mut proc.is_ignored, "Ignore");
            ui.label(&proc.exe_name);
            if ui.small_button("x").clicked() {
                remove_proc_idx = Some(i);
            }
        });
    }
    if let Some(i) = remove_proc_idx {
        profile.processes.remove(i);
    }

    ui.horizontal(|ui| {
        ui.text_edit_singleline(new_process_input);
        if ui.button("Add Process").clicked() && !new_process_input.is_empty() {
            let exe = new_process_input.clone();
            profile.processes.push(ProcessAction {
                exe_name: exe.clone(),
                display_name: exe,
                kill_on_launch: true,
                is_ignored: false,
                relaunch_after: false,
                captured_cmdline: None,
                captured_working_dir: None,
            });
            new_process_input.clear();
        }
    });
}
