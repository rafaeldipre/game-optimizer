use crate::core::errors::{AppError, AppResult};
use crate::core::model::KilledProcessRecord;
use crate::security::guards::{is_protected_process, is_warned_process};
use windows_sys::Win32::Foundation::{CloseHandle, FALSE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, TerminateProcess, PROCESS_NAME_WIN32,
    PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE,
};

pub struct ProcessInfo {
    pub pid: u32,
    pub exe_name: String,
    pub exe_path: Option<String>,
}

/// Enumerate all running processes
pub fn enumerate_processes() -> Vec<ProcessInfo> {
    let mut result = Vec::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return result;
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
                let exe_name = String::from_utf16_lossy(&entry.szExeFile[..name_end]);
                let pid = entry.th32ProcessID;

                let exe_path = get_process_path(pid);

                result.push(ProcessInfo {
                    pid,
                    exe_name,
                    exe_path,
                });

                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
    }

    result
}

fn get_process_path(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, pid);
        if handle == 0 {
            return None;
        }

        let mut buf = [0u16; 32768];
        let mut size = buf.len() as u32;

        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);

        if ok != 0 {
            Some(String::from_utf16_lossy(&buf[..size as usize]))
        } else {
            None
        }
    }
}

/// Find all PIDs for a given exe name (case-insensitive)
pub fn find_pids_by_name(exe_name: &str) -> Vec<u32> {
    enumerate_processes()
        .into_iter()
        .filter(|p| p.exe_name.eq_ignore_ascii_case(exe_name))
        .map(|p| p.pid)
        .collect()
}

/// Kill all processes with the given exe name.
/// Returns a record of each killed process for potential relaunch.
pub fn kill_processes_by_name(
    exe_name: &str,
    should_relaunch: bool,
) -> AppResult<Vec<KilledProcessRecord>> {
    // Security guard
    if is_protected_process(exe_name) {
        tracing::warn!(
            "[SECURITY] Attempt to kill protected process '{}' blocked",
            exe_name
        );
        crate::logging::logger::add_to_buffer(
            "SECURITY",
            &format!("Attempt to kill protected process '{}' blocked", exe_name),
        );
        return Err(AppError::SecurityGuard(format!(
            "Process '{}' is protected",
            exe_name
        )));
    }

    if is_warned_process(exe_name) {
        tracing::warn!(
            "[WARN] Killing '{}' — this process requires caution",
            exe_name
        );
        crate::logging::logger::add_to_buffer(
            "WARN",
            &format!("Killing '{}' — this process requires caution", exe_name),
        );
    }

    let processes = enumerate_processes();
    let mut records = Vec::new();

    for proc in processes
        .iter()
        .filter(|p| p.exe_name.eq_ignore_ascii_case(exe_name))
    {
        let cmdline = None;
        let working_dir = None;

        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, FALSE, proc.pid);
            if handle != 0 {
                TerminateProcess(handle, 0);
                CloseHandle(handle);
            }
        }

        tracing::info!("[INFO] Process {} (PID {}) terminated", exe_name, proc.pid);
        crate::logging::logger::add_to_buffer(
            "INFO",
            &format!("Process {} (PID {}) terminated", exe_name, proc.pid),
        );

        records.push(KilledProcessRecord {
            exe_name: exe_name.to_string(),
            pid: proc.pid,
            cmdline,
            working_dir,
            should_relaunch,
        });
    }

    if records.is_empty() {
        tracing::warn!(
            "[WARN] Process '{}' not found when attempting to kill",
            exe_name
        );
        crate::logging::logger::add_to_buffer(
            "WARN",
            &format!("Process '{}' not found when attempting to kill", exe_name),
        );
    }

    Ok(records)
}

/// Return a deduplicated, sorted list of (exe_name, first_pid) for all running
/// processes. Suitable for display in the process picker UI.
pub fn list_running_processes_snapshot() -> Vec<(String, u32)> {
    let mut seen: std::collections::HashMap<String, (String, u32)> =
        std::collections::HashMap::new();
    for p in enumerate_processes() {
        if p.pid == 0 || p.pid == 4 {
            continue;
        }
        let key = p.exe_name.to_lowercase();
        seen.entry(key).or_insert((p.exe_name, p.pid));
    }
    let mut list: Vec<(String, u32)> = seen
        .into_values()
        .collect();
    list.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    list
}

pub fn relaunch_process(record: &KilledProcessRecord) -> AppResult<()> {
    if let Some(ref _cmdline) = record.cmdline {
        let mut cmd = std::process::Command::new(&record.exe_name);
        if let Some(ref wd) = record.working_dir {
            cmd.current_dir(wd);
        }
        cmd.spawn().map_err(|e| AppError::ProcessError {
            process: record.exe_name.clone(),
            message: format!("Failed to relaunch: {}", e),
        })?;
        tracing::info!("[INFO] Process '{}' relaunched", record.exe_name);
        crate::logging::logger::add_to_buffer(
            "INFO",
            &format!("Process '{}' relaunched", record.exe_name),
        );
    }
    Ok(())
}
