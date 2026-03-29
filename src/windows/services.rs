use crate::core::errors::{AppError, AppResult};
use crate::security::guards::is_protected_service;
use windows_sys::Win32::System::Services::{
    CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, QueryServiceStatus,
    StartServiceW, SC_MANAGER_ALL_ACCESS, SERVICE_CONTROL_STOP,
    SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_START, SERVICE_STATUS, SERVICE_STOP,
};

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn query_service_running(service_name: &str) -> AppResult<bool> {
    unsafe {
        let scm = OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_ALL_ACCESS);
        if scm == 0 {
            return Err(AppError::ServiceError {
                service: service_name.to_string(),
                message: "OpenSCManager failed".to_string(),
            });
        }

        let name_w = to_wide(service_name);
        let svc = OpenServiceW(scm, name_w.as_ptr(), SERVICE_QUERY_STATUS);
        if svc == 0 {
            CloseServiceHandle(scm);
            return Err(AppError::ServiceError {
                service: service_name.to_string(),
                message: "OpenService failed".to_string(),
            });
        }

        let mut status: SERVICE_STATUS = std::mem::zeroed();
        let ok = QueryServiceStatus(svc, &mut status);
        CloseServiceHandle(svc);
        CloseServiceHandle(scm);

        if ok != 0 {
            Ok(status.dwCurrentState == SERVICE_RUNNING)
        } else {
            Err(AppError::ServiceError {
                service: service_name.to_string(),
                message: "QueryServiceStatus failed".to_string(),
            })
        }
    }
}

pub fn stop_service(service_name: &str) -> AppResult<()> {
    if is_protected_service(service_name) {
        tracing::warn!(
            "[SECURITY] Attempt to stop protected service '{}' blocked",
            service_name
        );
        crate::logging::logger::add_to_buffer(
            "SECURITY",
            &format!(
                "Attempt to stop protected service '{}' blocked",
                service_name
            ),
        );
        return Err(AppError::SecurityGuard(format!(
            "Service '{}' is protected",
            service_name
        )));
    }

    unsafe {
        let scm = OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_ALL_ACCESS);
        if scm == 0 {
            return Err(AppError::ServiceError {
                service: service_name.to_string(),
                message: "OpenSCManager failed".to_string(),
            });
        }

        let name_w = to_wide(service_name);
        let svc = OpenServiceW(
            scm,
            name_w.as_ptr(),
            SERVICE_STOP | SERVICE_QUERY_STATUS,
        );
        if svc == 0 {
            CloseServiceHandle(scm);
            return Ok(()); // Service may not exist
        }

        let mut status: SERVICE_STATUS = std::mem::zeroed();
        ControlService(svc, SERVICE_CONTROL_STOP, &mut status);
        CloseServiceHandle(svc);
        CloseServiceHandle(scm);
    }

    tracing::info!("[INFO] Service '{}' stop requested", service_name);
    crate::logging::logger::add_to_buffer(
        "INFO",
        &format!("Service '{}' stop requested", service_name),
    );
    Ok(())
}

pub fn start_service(service_name: &str) -> AppResult<()> {
    unsafe {
        let scm = OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_ALL_ACCESS);
        if scm == 0 {
            return Err(AppError::ServiceError {
                service: service_name.to_string(),
                message: "OpenSCManager failed".to_string(),
            });
        }

        let name_w = to_wide(service_name);
        let svc = OpenServiceW(scm, name_w.as_ptr(), SERVICE_START);
        if svc == 0 {
            CloseServiceHandle(scm);
            return Ok(());
        }

        StartServiceW(svc, 0, std::ptr::null());
        CloseServiceHandle(svc);
        CloseServiceHandle(scm);
    }

    tracing::info!("[INFO] Service '{}' start requested", service_name);
    crate::logging::logger::add_to_buffer(
        "INFO",
        &format!("Service '{}' start requested", service_name),
    );
    Ok(())
}
