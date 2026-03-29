use crate::core::errors::AppResult;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows_sys::Win32::Foundation::{CloseHandle, FALSE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};

/// Monitor `exe_name` starting from `initial_pid`.
/// Handles the launcher → game re-launch pattern:
///   1. Wait for `initial_pid` to exit.
///   2. Wait `grace_secs` for a successor process with the same name.
///   3. If one appears, switch monitoring to it (and return its PID so the caller
///      can re-apply priority/affinity).
///   4. Repeat until the name disappears completely.
///
/// Returns `Ok(final_pid)` — the PID of the last monitored instance.
pub async fn wait_for_exe_exit(
    initial_pid: u32,
    exe_name: &str,
    stop_signal: Arc<AtomicBool>,
    grace_secs: u64,
) -> AppResult<u32> {
    let mut current_pid = initial_pid;

    crate::logging::logger::add_to_buffer(
        "DEBUG",
        &format!("Monitoring '{}' (PID {})", exe_name, current_pid),
    );
    tracing::debug!("[DEBUG] Monitoring '{}' (PID {})", exe_name, current_pid);

    loop {
        // Wait for the current PID to exit
        wait_for_pid_exit(current_pid, &stop_signal).await?;

        if stop_signal.load(Ordering::Relaxed) {
            crate::logging::logger::add_to_buffer("INFO", "Monitor stopped by user signal");
            return Ok(current_pid);
        }

        // Process exited — wait the grace period for a possible re-launch
        crate::logging::logger::add_to_buffer(
            "INFO",
            &format!(
                "PID {} ('{}') exited — waiting {}s for possible re-launch (launcher → game)",
                current_pid, exe_name, grace_secs
            ),
        );
        tracing::info!(
            "[INFO] PID {} exited — waiting {}s for re-launch check",
            current_pid, grace_secs
        );

        for _ in 0..grace_secs {
            if stop_signal.load(Ordering::Relaxed) {
                return Ok(current_pid);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // Look for a successor with the same exe name (different PID)
        let new_pid = find_pids_by_name(exe_name)
            .into_iter()
            .find(|&p| p != current_pid);

        match new_pid {
            Some(pid) => {
                crate::logging::logger::add_to_buffer(
                    "INFO",
                    &format!(
                        "'{}' re-launched as PID {} (launcher → game). Resuming monitor.",
                        exe_name, pid
                    ),
                );
                tracing::info!(
                    "[INFO] '{}' re-launched as PID {} — resuming monitor",
                    exe_name, pid
                );
                current_pid = pid;
                // Continue outer loop — will now monitor the new PID
            }
            None => {
                // No successor — game truly exited
                crate::logging::logger::add_to_buffer(
                    "INFO",
                    &format!("'{}' exited for good (PID {}). Starting restore.", exe_name, current_pid),
                );
                tracing::info!("[INFO] '{}' exited. Starting restore.", exe_name);
                return Ok(current_pid);
            }
        }
    }
}

/// Block (async) until the given PID is no longer running.
/// Polls every 2 s so the stop-signal can be checked.
async fn wait_for_pid_exit(pid: u32, stop_signal: &Arc<AtomicBool>) -> AppResult<()> {
    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, FALSE, pid) };
    if handle == 0 {
        // Process already gone or inaccessible — treat as exited
        return Ok(());
    }

    loop {
        if stop_signal.load(Ordering::Relaxed) {
            unsafe { CloseHandle(handle); }
            return Ok(());
        }

        let result = unsafe { WaitForSingleObject(handle, 2000) };
        if result == WAIT_OBJECT_0 {
            unsafe { CloseHandle(handle); }
            return Ok(());
        }
        // WAIT_TIMEOUT — still running; yield to tokio before next check
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

/// Return all PIDs whose exe filename matches (case-insensitive).
pub fn find_pids_by_name(exe_name: &str) -> Vec<u32> {
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let mut pids = Vec::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return pids;
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let name_end = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..name_end]);
                if name.eq_ignore_ascii_case(exe_name) {
                    pids.push(entry.th32ProcessID);
                }
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
    }
    pids
}
