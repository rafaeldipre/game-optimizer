use crate::app::AppState;
use crate::core::model::{
    GpuPreference, PreLaunchProgram, ProcessAction, ProcessPriority, Profile, ServiceAction,
};
use crate::persistence::profiles as profiles_persistence;
use chrono::Utc;
use egui::RichText;
use uuid::Uuid;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    // ── Process picker modal (rendered FIRST so it floats above everything) ─────
    if state.show_process_picker {
        let mut close_picker = false;
        let mut picked: Option<String> = None;

        egui::Window::new("Pick a Running Process")
            .collapsible(false)
            .resizable(true)
            .default_size([420.0, 480.0])
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    ui.text_edit_singleline(&mut state.process_picker_filter);
                    if ui.button("Refresh").clicked() {
                        #[cfg(target_os = "windows")]
                        {
                            state.process_picker_list =
                                crate::windows::processes::list_running_processes_snapshot();
                        }
                    }
                    if ui.button("✖ Close").clicked() {
                        close_picker = true;
                    }
                });
                ui.separator();

                let filter = state.process_picker_filter.to_lowercase();
                egui::ScrollArea::vertical()
                    .max_height(380.0)
                    .show(ui, |ui| {
                        egui::Grid::new("picker_grid")
                            .num_columns(3)
                            .striped(true)
                            .spacing([6.0, 2.0])
                            .show(ui, |ui| {
                                for (name, pid) in &state.process_picker_list {
                                    if !filter.is_empty()
                                        && !name.to_lowercase().contains(&filter)
                                    {
                                        continue;
                                    }
                                    ui.label(name.as_str());
                                    ui.label(
                                        RichText::new(format!("PID {}", pid))
                                            .weak(),
                                    );
                                    if ui.small_button("+ Add").clicked() {
                                        picked = Some(name.clone());
                                    }
                                    ui.end_row();
                                }
                            });
                    });
            });

        // Apply pick outside the window closure to avoid borrow issues
        if let Some(exe_name) = picked {
            if let Some(ref mut edit) = state.profile_edit_state {
                // Avoid duplicates
                if !edit.processes.iter().any(|p| {
                    p.exe_name.eq_ignore_ascii_case(&exe_name)
                }) {
                    edit.processes.push(ProcessAction {
                        exe_name: exe_name.clone(),
                        display_name: exe_name,
                        kill_on_launch: true,
                        is_ignored: false,
                        relaunch_after: false,
                        captured_cmdline: None,
                        captured_working_dir: None,
                    });
                }
            }
        }
        if close_picker {
            state.show_process_picker = false;
            state.process_picker_filter.clear();
        }
    }

    // ── Main layout ──────────────────────────────────────────────────────────────
    ui.heading("Profiles");
    ui.separator();

    ui.horizontal(|ui| {
        // Left panel: profile list
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
                        let resp = ui.selectable_label(selected, &name);
                        if resp.clicked() {
                            state.selected_profile_index = Some(i);
                        }
                        // Double-click opens editor directly
                        if resp.double_clicked() {
                            state.selected_profile_index = Some(i);
                            state.profile_edit_state = Some(state.profiles[i].clone());
                            state.is_editing_profile = true;
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
            let mut open_picker = false;

            if let Some(ref mut edit) = state.profile_edit_state {
                let mut new_svc = std::mem::take(&mut state.new_service_input);
                let mut new_proc = std::mem::take(&mut state.new_process_input);
                let mut new_pre_exe = std::mem::take(&mut state.new_prelaunch_exe);
                let mut new_pre_name = std::mem::take(&mut state.new_prelaunch_name);
                let mut new_pre_args = std::mem::take(&mut state.new_prelaunch_args);
                let mut new_pre_wait = state.new_prelaunch_wait_ms;

                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_profile_editor(
                        ui,
                        edit,
                        &mut new_svc,
                        &mut new_proc,
                        &mut new_pre_exe,
                        &mut new_pre_name,
                        &mut new_pre_args,
                        &mut new_pre_wait,
                        &mut open_picker,
                    );

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

                state.new_service_input = new_svc;
                state.new_process_input = new_proc;
                state.new_prelaunch_exe = new_pre_exe;
                state.new_prelaunch_name = new_pre_name;
                state.new_prelaunch_args = new_pre_args;
                state.new_prelaunch_wait_ms = new_pre_wait;
            }

            if open_picker {
                // Snapshot running processes right now
                #[cfg(target_os = "windows")]
                {
                    state.process_picker_list =
                        crate::windows::processes::list_running_processes_snapshot();
                }
                state.process_picker_filter.clear();
                state.show_process_picker = true;
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
                state.new_prelaunch_exe.clear();
                state.new_prelaunch_name.clear();
                state.new_prelaunch_args.clear();
                state.new_prelaunch_wait_ms = 2000;
            }
            if should_cancel {
                state.is_editing_profile = false;
                state.profile_edit_state = None;
                state.new_service_input.clear();
                state.new_process_input.clear();
                state.new_prelaunch_exe.clear();
                state.new_prelaunch_name.clear();
                state.new_prelaunch_args.clear();
                state.new_prelaunch_wait_ms = 2000;
            }
        } else if let Some(idx) = state.selected_profile_index {
            if let Some(profile) = state.profiles.get(idx) {
                ui.vertical(|ui| {
                    ui.label(RichText::new(&profile.friendly_name).heading());
                    ui.label(format!("ID: {}", profile.id));
                    ui.label(format!("Executable: {}", profile.executable_path));
                    ui.label(format!("Priority: {}", profile.process_priority));
                    ui.label(format!("GPU: {}", profile.gpu_preference));
                    ui.label(format!("Services to stop: {}", profile.services.len()));
                    ui.label(format!("Programs to close: {}", profile.processes.len()));
                    ui.label(format!("Pre-launch programs: {}", profile.pre_launch_programs.len()));
                    if !profile.notes.is_empty() {
                        ui.separator();
                        ui.label(RichText::new("Notes:").strong());
                        ui.label(&profile.notes);
                    }
                });
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(RichText::new("Select a profile from the list").strong());
                ui.label(RichText::new("Single-click to select  •  Double-click to edit").weak());
                ui.add_space(8.0);
                ui.label(RichText::new("or click  New  to create one").weak());
            });
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn render_profile_editor(
    ui: &mut egui::Ui,
    profile: &mut Profile,
    new_service_input: &mut String,
    new_process_input: &mut String,
    new_pre_exe: &mut String,
    new_pre_name: &mut String,
    new_pre_args: &mut String,
    new_pre_wait: &mut u64,
    open_picker: &mut bool,
) {
    ui.label(RichText::new("Edit Profile").strong());
    ui.separator();

    // ── Basic fields ─────────────────────────────────────────────────────────────
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

            ui.label("Trim background RAM:");
            ui.checkbox(&mut profile.memory_optimization.trim_other_processes, "");
            ui.end_row();

            ui.label("Delays (ms):");
            ui.horizontal(|ui| {
                ui.label("Services:");
                ui.add(
                    egui::DragValue::new(&mut profile.delays.after_services_ms)
                        .clamp_range(0u64..=30000u64),
                );
                ui.label("Processes:");
                ui.add(
                    egui::DragValue::new(&mut profile.delays.after_processes_ms)
                        .clamp_range(0u64..=30000u64),
                );
                ui.label("After launch:");
                ui.add(
                    egui::DragValue::new(&mut profile.delays.after_launch_ms)
                        .clamp_range(0u64..=60000u64),
                );
            });
            ui.end_row();

            ui.label("Relaunch grace (s):");
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut profile.delays.relaunch_grace_secs)
                        .clamp_range(5u64..=300u64),
                );
                ui.label(RichText::new("seconds to wait for launcher → game transition").weak());
            });
            ui.end_row();

            ui.label("Notes:");
            ui.text_edit_multiline(&mut profile.notes);
            ui.end_row();
        });

    // ── Services to stop ─────────────────────────────────────────────────────────
    ui.add_space(10.0);
    ui.label(RichText::new("🛑  Services to Stop").strong());
    ui.separator();

    let mut remove_svc_idx: Option<usize> = None;
    egui::Grid::new("svc_grid")
        .num_columns(4)
        .striped(true)
        .spacing([6.0, 2.0])
        .show(ui, |ui| {
            for (i, svc) in profile.services.iter_mut().enumerate() {
                ui.checkbox(&mut svc.stop_on_launch, "Stop");
                ui.label(&svc.service_name);
                if !svc.display_name.is_empty() && svc.display_name != svc.service_name {
                    ui.label(
                        RichText::new(format!("({})", svc.display_name)).weak(),
                    );
                } else {
                    ui.label("");
                }
                if ui.small_button("✖").clicked() {
                    remove_svc_idx = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove_svc_idx {
        profile.services.remove(i);
    }

    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(new_service_input)
                .hint_text("Service name, e.g. SysMain"),
        );
        if ui.button("+ Add Service").clicked() && !new_service_input.is_empty() {
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

    // ── Programs to close before launch ──────────────────────────────────────────
    ui.add_space(10.0);
    ui.label(RichText::new("❌  Programs to Close Before Launch").strong());
    ui.label(
        RichText::new("These programs will be killed when the game starts and optionally restarted when it closes.")
            .weak()
            .small(),
    );
    ui.separator();

    let mut remove_proc_idx: Option<usize> = None;
    egui::Grid::new("proc_grid")
        .num_columns(5)
        .striped(true)
        .spacing([6.0, 2.0])
        .show(ui, |ui| {
            ui.label(RichText::new("Program").strong());
            ui.label(RichText::new("Kill").strong());
            ui.label(RichText::new("Relaunch after").strong());
            ui.label(RichText::new("Ignore").strong());
            ui.label("");
            ui.end_row();

            for (i, proc) in profile.processes.iter_mut().enumerate() {
                ui.label(&proc.exe_name);
                ui.checkbox(&mut proc.kill_on_launch, "");
                ui.checkbox(&mut proc.relaunch_after, "");
                ui.checkbox(&mut proc.is_ignored, "");
                if ui.small_button("✖").clicked() {
                    remove_proc_idx = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove_proc_idx {
        profile.processes.remove(i);
    }

    // Add controls row
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(new_process_input)
                .hint_text("e.g. Discord.exe")
                .desired_width(160.0),
        );
        if ui.button("+ Add by name").clicked() && !new_process_input.is_empty() {
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
        if ui.button("📂 Browse .exe...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Executable", &["exe"])
                .pick_file()
            {
                let exe_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !exe_name.is_empty()
                    && !profile
                        .processes
                        .iter()
                        .any(|p| p.exe_name.eq_ignore_ascii_case(&exe_name))
                {
                    profile.processes.push(ProcessAction {
                        exe_name: exe_name.clone(),
                        display_name: exe_name,
                        kill_on_launch: true,
                        is_ignored: false,
                        relaunch_after: false,
                        captured_cmdline: None,
                        captured_working_dir: None,
                    });
                }
            }
        }
        if ui.button("🔍 Pick from Running...").clicked() {
            *open_picker = true;
        }
    });

    // ── Programs to launch before game ───────────────────────────────────────────
    ui.add_space(10.0);
    ui.label(RichText::new("▶  Programs to Launch Before Game").strong());
    ui.label(
        RichText::new(
            "These programs will be started automatically before the game launches.",
        )
        .weak()
        .small(),
    );
    ui.separator();

    let mut remove_pre_idx: Option<usize> = None;
    egui::Grid::new("prelaunch_grid")
        .num_columns(5)
        .striped(true)
        .spacing([6.0, 2.0])
        .show(ui, |ui| {
            ui.label(RichText::new("Name").strong());
            ui.label(RichText::new("Executable").strong());
            ui.label(RichText::new("Args").strong());
            ui.label(RichText::new("Wait (ms)").strong());
            ui.label("");
            ui.end_row();

            for (i, pre) in profile.pre_launch_programs.iter_mut().enumerate() {
                ui.text_edit_singleline(&mut pre.display_name);
                ui.label(&pre.exe_path);
                let mut args = pre.args.clone().unwrap_or_default();
                ui.text_edit_singleline(&mut args);
                pre.args = if args.is_empty() { None } else { Some(args) };
                ui.add(
                    egui::DragValue::new(&mut pre.wait_ms)
                        .clamp_range(0u64..=30000u64),
                );
                if ui.small_button("✖").clicked() {
                    remove_pre_idx = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove_pre_idx {
        profile.pre_launch_programs.remove(i);
    }

    // Add form for new pre-launch program
    ui.add_space(4.0);
    ui.group(|ui| {
        ui.label(RichText::new("Add pre-launch program:").small().strong());
        egui::Grid::new("prelaunch_add_grid")
            .num_columns(2)
            .spacing([6.0, 4.0])
            .show(ui, |ui| {
                ui.label("Display name:");
                ui.text_edit_singleline(new_pre_name);
                ui.end_row();

                ui.label("Executable:");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(new_pre_exe)
                            .desired_width(220.0)
                            .hint_text("Full path to .exe"),
                    );
                    if ui.button("📂 Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Executable", &["exe"])
                            .pick_file()
                        {
                            *new_pre_exe = path.to_string_lossy().to_string();
                            if new_pre_name.is_empty() {
                                if let Some(stem) = path.file_stem() {
                                    *new_pre_name = stem.to_string_lossy().to_string();
                                }
                            }
                        }
                    }
                });
                ui.end_row();

                ui.label("Arguments:");
                ui.text_edit_singleline(new_pre_args);
                ui.end_row();

                ui.label("Wait after start (ms):");
                ui.add(
                    egui::DragValue::new(new_pre_wait)
                        .clamp_range(0u64..=30000u64),
                );
                ui.end_row();
            });

        if ui.button("+ Add Program").clicked() && !new_pre_exe.is_empty() {
            let name = if new_pre_name.is_empty() {
                std::path::Path::new(new_pre_exe.as_str())
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| new_pre_exe.clone())
            } else {
                new_pre_name.clone()
            };
            profile.pre_launch_programs.push(PreLaunchProgram {
                display_name: name,
                exe_path: new_pre_exe.clone(),
                args: if new_pre_args.is_empty() {
                    None
                } else {
                    Some(new_pre_args.clone())
                },
                working_dir: None,
                wait_ms: *new_pre_wait,
            });
            new_pre_exe.clear();
            new_pre_name.clear();
            new_pre_args.clear();
            *new_pre_wait = 2000;
        }
    });
}
