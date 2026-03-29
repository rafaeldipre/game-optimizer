use crate::core::errors::{AppError, AppResult};
use crate::core::model::{GpuPreference, SessionPhase, SessionState, SystemSnapshot};
use crate::persistence::session::save_session;
use chrono::Utc;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(target_os = "windows")]
use crate::windows::{elevation::check_elevation, gpu, memory, monitor, priority, processes, services};

pub struct Optimizer;

impl Optimizer {
    pub async fn launch_profile(
        profile: &crate::core::model::Profile,
        shared_session: Arc<Mutex<SessionState>>,
        stop_signal: Arc<AtomicBool>,
    ) -> AppResult<()> {
        #[cfg(target_os = "windows")]
        check_elevation()?;

        if !Path::new(&profile.executable_path).exists() {
            return Err(AppError::ExecutableNotFound(profile.executable_path.clone()));
        }

        tracing::info!("[INFO] Profile '{}' started", profile.friendly_name);
        crate::logging::logger::add_to_buffer("INFO", &format!("Profile '{}' started", profile.friendly_name));

        let mut snapshot = SystemSnapshot { captured_at: Utc::now(), ..Default::default() };

        #[cfg(target_os = "windows")]
        {
            for svc in &profile.services {
                if svc.stop_on_launch {
                    match services::query_service_running(&svc.service_name) {
                        Ok(was_running) => { snapshot.services.insert(svc.service_name.clone(), was_running); }
                        Err(e) => { tracing::warn!("[WARN] Could not query service '{}': {}", svc.service_name, e); }
                    }
                }
            }
            let exe_filename = Path::new(&profile.executable_path).file_name().and_then(|n| n.to_str()).unwrap_or("");
            snapshot.gpu_preference_before = gpu::read_gpu_preference(exe_filename);
        }

        // PreLaunch phase
        {
            let mut sess = shared_session.lock().unwrap();
            sess.profile_id = Some(profile.id);
            sess.profile_name = Some(profile.friendly_name.clone());
            sess.phase = SessionPhase::PreLaunch;
            sess.started_at = Some(Utc::now());
            sess.pre_session_snapshot = Some(snapshot.clone());
            save_session(&sess)?;
        }

        #[cfg(target_os = "windows")]
        {
            // Stop services
            for svc in &profile.services {
                if svc.stop_on_launch {
                    let was_running = snapshot.services.get(&svc.service_name).copied().unwrap_or(false);
                    if was_running {
                        match services::stop_service(&svc.service_name) {
                            Ok(_) => {
                                tracing::info!("[INFO] Service {} stopped (was Running)", svc.service_name);
                                crate::logging::logger::add_to_buffer("INFO", &format!("Service {} stopped (was Running)", svc.service_name));
                            }
                            Err(AppError::SecurityGuard(_)) => {}
                            Err(e) => {
                                tracing::warn!("[WARN] Could not stop service '{}': {}", svc.service_name, e);
                                crate::logging::logger::add_to_buffer("WARN", &format!("Could not stop service '{}': {}", svc.service_name, e));
                            }
                        }
                    } else {
                        tracing::warn!("[WARN] Service {} already stopped, skipped", svc.service_name);
                        crate::logging::logger::add_to_buffer("WARN", &format!("Service {} already stopped, skipped", svc.service_name));
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(profile.delays.after_services_ms)).await;

            // Kill processes
            for proc_action in &profile.processes {
                if proc_action.kill_on_launch && !proc_action.is_ignored {
                    match processes::kill_processes_by_name(&proc_action.exe_name, proc_action.relaunch_after) {
                        Ok(records) => {
                            for rec in records {
                                snapshot.killed_processes.push(rec);
                            }
                        }
                        Err(AppError::SecurityGuard(_)) => {}
                        Err(e) => { tracing::warn!("[WARN] Error killing '{}': {}", proc_action.exe_name, e); }
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(profile.delays.after_processes_ms)).await;

            // Set GPU preference
            let exe_filename = Path::new(&profile.executable_path).file_name().and_then(|n| n.to_str()).unwrap_or("");
            if profile.gpu_preference != GpuPreference::Default {
                match gpu::set_gpu_preference(exe_filename, &profile.gpu_preference) {
                    Ok(prev) => {
                        snapshot.gpu_preference_before = prev;
                        shared_session.lock().unwrap().pre_session_snapshot = Some(snapshot.clone());
                    }
                    Err(e) => { tracing::warn!("[WARN] Could not set GPU preference: {}", e); }
                }
            }
        }

        // Launch target process
        let mut cmd = std::process::Command::new(&profile.executable_path);
        if let Some(ref args) = profile.launch_args {
            if !args.is_empty() {
                cmd.args(args.split_whitespace());
            }
        }
        if let Some(ref workdir) = profile.working_directory {
            cmd.current_dir(workdir);
        }

        let child = cmd.spawn().map_err(|e| AppError::ProcessError {
            process: profile.executable_path.clone(),
            message: format!("Failed to launch: {}", e),
        })?;

        let initial_pid = child.id();
        tracing::info!("[INFO] {} launched (PID {})", profile.executable_path, initial_pid);
        crate::logging::logger::add_to_buffer("INFO", &format!("{} launched (initial PID {})", profile.executable_path, initial_pid));

        // Get the exe filename for name-based monitoring
        let exe_filename = std::path::Path::new(&profile.executable_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Running phase
        {
            let mut sess = shared_session.lock().unwrap();
            sess.target_pid = Some(initial_pid);
            sess.phase = SessionPhase::Running;
            sess.pre_session_snapshot = Some(snapshot.clone());
            save_session(&sess)?;
        }

        #[cfg(target_os = "windows")]
        {
            // Helper closure to apply priority/affinity/working-set to a PID
            let apply_settings = |pid: u32| {
                if let Err(e) = priority::set_process_priority(pid, &profile.process_priority) {
                    tracing::error!("[ERROR] Could not set priority for PID {}: {}", pid, e);
                    crate::logging::logger::add_to_buffer("ERROR", &format!("Could not set priority for PID {}: {}", pid, e));
                }
                if let Err(e) = priority::set_process_affinity(pid, &profile.cpu_affinity) {
                    tracing::warn!("[WARN] Could not set affinity for PID {}: {}", pid, e);
                }
                if let Err(e) = memory::set_working_set(pid, &profile.memory_optimization) {
                    tracing::warn!("[WARN] Could not set working set for PID {}: {}", pid, e);
                }
            };

            // Wait for the initial process + any after-launch delay, then apply settings
            tokio::time::sleep(tokio::time::Duration::from_millis(profile.delays.after_launch_ms)).await;
            apply_settings(initial_pid);

            if profile.memory_optimization.trim_other_processes {
                tracing::warn!("[WARN] trim_other_processes is enabled — may cause paging slowness");
                crate::logging::logger::add_to_buffer("WARN", "trim_other_processes active — may cause paging slowness");
                memory::trim_background_processes(initial_pid);
            }

            // Monitor by exe NAME so re-launches (launcher → game) are handled.
            // wait_for_exe_exit returns the PID of the last observed instance.
            let final_pid = monitor::wait_for_exe_exit(initial_pid, &exe_filename, stop_signal, profile.delays.relaunch_grace_secs).await?;

            // If the game re-launched (new PID ≠ initial), re-apply settings to the real game process.
            if final_pid != initial_pid {
                crate::logging::logger::add_to_buffer(
                    "INFO",
                    &format!("Re-applying settings to game PID {} (was launcher PID {})", final_pid, initial_pid),
                );
                apply_settings(final_pid);
            }

            // Update session with final PID
            shared_session.lock().unwrap().target_pid = Some(final_pid);
        }

        #[cfg(not(target_os = "windows"))]
        { drop(child); }

        // Restoring phase
        {
            let mut sess = shared_session.lock().unwrap();
            sess.phase = SessionPhase::Restoring;
            save_session(&sess)?;
        }

        let snap = shared_session.lock().unwrap().pre_session_snapshot.clone().unwrap_or_default();
        Self::restore_system(&snap, profile).await?;

        // Completed
        {
            let mut sess = shared_session.lock().unwrap();
            sess.phase = SessionPhase::Completed;
            sess.ended_at = Some(Utc::now());
            save_session(&sess)?;
        }

        tracing::info!("[INFO] Restore completed for profile '{}'", profile.friendly_name);
        crate::logging::logger::add_to_buffer("INFO", "Restore completed");
        Ok(())
    }

    pub async fn restore_system(snapshot: &SystemSnapshot, profile: &crate::core::model::Profile) -> AppResult<()> {
        #[cfg(target_os = "windows")]
        {
            let exe_filename = std::path::Path::new(&profile.executable_path)
                .file_name().and_then(|n| n.to_str()).unwrap_or("");
            if let Err(e) = gpu::restore_gpu_preference(exe_filename, snapshot.gpu_preference_before.as_deref()) {
                tracing::warn!("[WARN] Could not restore GPU preference: {}", e);
            }

            for (service_name, was_running) in &snapshot.services {
                if *was_running {
                    match services::start_service(service_name) {
                        Ok(_) => {
                            tracing::info!("[INFO] Service {} restored (started)", service_name);
                            crate::logging::logger::add_to_buffer("INFO", &format!("Service {} restored (started)", service_name));
                        }
                        Err(e) => {
                            tracing::warn!("[WARN] Could not restore service '{}': {}", service_name, e);
                            crate::logging::logger::add_to_buffer("WARN", &format!("Could not restore service '{}': {}", service_name, e));
                        }
                    }
                }
            }

            for record in &snapshot.killed_processes {
                if record.should_relaunch && record.cmdline.is_some() {
                    if let Err(e) = processes::relaunch_process(record) {
                        tracing::warn!("[WARN] Could not relaunch '{}': {}", record.exe_name, e);
                    }
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        { let _ = snapshot; let _ = profile; }

        Ok(())
    }
}
