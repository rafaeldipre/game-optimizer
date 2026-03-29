use crate::core::errors::{AppError, AppResult};
use crate::core::model::MemoryOptimization;
use windows_sys::Win32::Foundation::{CloseHandle, FALSE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{EmptyWorkingSet, GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::{
    OpenProcess, SetProcessWorkingSetSize, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA,
};

/// Only trim background processes that consume more than this threshold.
/// Trimming small processes wastes disk I/O without meaningful RAM recovery.
const MIN_TRIM_BYTES: usize = 100 * 1024 * 1024; // 100 MB

pub fn set_working_set(pid: u32, opts: &MemoryOptimization) -> AppResult<()> {
    let min_bytes = match opts.target_min_working_set_bytes {
        Some(b) => b as usize,
        None => return Ok(()),
    };

    unsafe {
        let handle = OpenProcess(PROCESS_SET_QUOTA | PROCESS_QUERY_INFORMATION, FALSE, pid);
        if handle == 0 {
            return Err(AppError::WindowsApi("OpenProcess for working set failed".to_string()));
        }
        // SetProcessWorkingSetSize(handle, min, max) — max = 4x min as a reasonable ceiling
        let ok = SetProcessWorkingSetSize(handle, min_bytes, min_bytes * 4);
        CloseHandle(handle);
        if ok == 0 {
            return Err(AppError::WindowsApi("SetProcessWorkingSetSize failed".to_string()));
        }
    }

    tracing::info!("[INFO] Working set hint set for PID {}: min={}MB", pid, min_bytes / 1024 / 1024);
    crate::logging::logger::add_to_buffer(
        "INFO",
        &format!("Working set hint set for PID {}: min={}MB", pid, min_bytes / 1024 / 1024),
    );
    Ok(())
}

/// Trim the working set of background processes that use more than MIN_TRIM_BYTES.
///
/// WARNING: Even with a threshold this causes disk I/O (paging). Use sparingly.
/// Do NOT call this during game loading — only after the game is fully loaded.
pub fn trim_background_processes(target_pid: u32) {
    use crate::security::guards::is_protected_process;

    let mut trimmed = 0u32;
    let mut skipped = 0u32;

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return;
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let pid = entry.th32ProcessID;
                if pid != target_pid && pid != 0 {
                    let name_end = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let exe_name = String::from_utf16_lossy(&entry.szExeFile[..name_end]);

                    if !is_protected_process(&exe_name) {
                        let handle = OpenProcess(
                            PROCESS_SET_QUOTA | PROCESS_QUERY_INFORMATION,
                            FALSE,
                            pid,
                        );
                        if handle != 0 {
                            // Check working set size before trimming
                            let mut pmc: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
                            pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                            let mem_ok = GetProcessMemoryInfo(handle, &mut pmc, pmc.cb);

                            if mem_ok != 0 && pmc.WorkingSetSize > MIN_TRIM_BYTES {
                                EmptyWorkingSet(handle);
                                trimmed += 1;
                                tracing::debug!(
                                    "[DEBUG] Trimmed {} (PID {}, WS={}MB)",
                                    exe_name, pid,
                                    pmc.WorkingSetSize / 1024 / 1024
                                );
                            } else {
                                skipped += 1;
                            }

                            CloseHandle(handle);
                        }
                    }
                }

                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
    }

    tracing::info!("[INFO] Background trim complete: {} trimmed, {} skipped (below {}MB)", trimmed, skipped, MIN_TRIM_BYTES / 1024 / 1024);
    crate::logging::logger::add_to_buffer(
        "INFO",
        &format!("Background trim: {} trimmed (>100MB), {} skipped", trimmed, skipped),
    );
}
