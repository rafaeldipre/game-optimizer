use crate::core::errors::{AppError, AppResult};
use crate::core::model::{CpuAffinity, ProcessPriority};
use windows_sys::Win32::Foundation::{CloseHandle, FALSE};
use windows_sys::Win32::System::Threading::{
    OpenProcess, SetPriorityClass, SetProcessAffinityMask, ABOVE_NORMAL_PRIORITY_CLASS,
    BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
    PROCESS_SET_INFORMATION, REALTIME_PRIORITY_CLASS,
};

pub fn set_process_priority(pid: u32, priority: &ProcessPriority) -> AppResult<()> {
    let priority_class = match priority {
        ProcessPriority::Idle => IDLE_PRIORITY_CLASS,
        ProcessPriority::BelowNormal => BELOW_NORMAL_PRIORITY_CLASS,
        ProcessPriority::Normal => NORMAL_PRIORITY_CLASS,
        ProcessPriority::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
        ProcessPriority::High => HIGH_PRIORITY_CLASS,
        ProcessPriority::Realtime => {
            tracing::warn!(
                "[SECURITY] Setting REALTIME priority for PID {} — system freeze risk",
                pid
            );
            crate::logging::logger::add_to_buffer(
                "SECURITY",
                &format!("Setting REALTIME priority for PID {}", pid),
            );
            REALTIME_PRIORITY_CLASS
        }
    };

    unsafe {
        let handle = OpenProcess(PROCESS_SET_INFORMATION, FALSE, pid);
        if handle == 0 {
            return Err(AppError::WindowsApi(
                "OpenProcess for priority failed".to_string(),
            ));
        }
        let ok = SetPriorityClass(handle, priority_class);
        CloseHandle(handle);
        if ok == 0 {
            return Err(AppError::WindowsApi("SetPriorityClass failed".to_string()));
        }
    }

    tracing::info!("[INFO] Priority {:?} set for PID {}", priority, pid);
    crate::logging::logger::add_to_buffer(
        "INFO",
        &format!("Priority {:?} set for PID {}", priority, pid),
    );
    Ok(())
}

pub fn set_process_affinity(pid: u32, affinity: &CpuAffinity) -> AppResult<()> {
    let mask = match affinity.mask {
        Some(m) => m,
        None => return Ok(()), // No affinity change requested
    };

    unsafe {
        let handle = OpenProcess(PROCESS_SET_INFORMATION, FALSE, pid);
        if handle == 0 {
            return Err(AppError::WindowsApi(
                "OpenProcess for affinity failed".to_string(),
            ));
        }
        let ok = SetProcessAffinityMask(handle, mask as usize);
        CloseHandle(handle);
        if ok == 0 {
            return Err(AppError::WindowsApi(
                "SetProcessAffinityMask failed".to_string(),
            ));
        }
    }

    tracing::info!(
        "[INFO] CPU affinity mask=0x{:X} set for PID {}",
        mask,
        pid,
    );
    crate::logging::logger::add_to_buffer(
        "INFO",
        &format!("CPU affinity mask=0x{:X} set for PID {}", mask, pid),
    );
    Ok(())
}

pub fn get_logical_processor_count() -> u32 {
    unsafe {
        let mut info: windows_sys::Win32::System::SystemInformation::SYSTEM_INFO =
            std::mem::zeroed();
        windows_sys::Win32::System::SystemInformation::GetSystemInfo(&mut info);
        info.dwNumberOfProcessors
    }
}
