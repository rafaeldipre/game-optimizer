/// Protected Windows services — NEVER stop these
const PROTECTED_SERVICES: &[&str] = &[
    "lsass",
    "services",
    "winlogon",
    "csrss",
    "smss",
    "wininit",
    "system",
    "registry",
    "WinDefend",
    "MpsSvc",
    "EventLog",
    "CryptSvc",
    "RpcSs",
    "DcomLaunch",
    "LSM",
    "SamSs",
    "VaultSvc",
    "Netlogon",
    "gpsvc",
    "ProfSvc",
    "UserManager",
];

/// Protected Windows processes — NEVER terminate these
const PROTECTED_PROCESSES: &[&str] = &[
    "lsass.exe",
    "csrss.exe",
    "winlogon.exe",
    "wininit.exe",
    "smss.exe",
    "svchost.exe",
    "system",
    "registry",
    "MsMpEng.exe",
    "dwm.exe",
];

/// Processes that trigger a warning log but are allowed
const WARNED_PROCESSES: &[&str] = &[
    "explorer.exe",
    "audiodg.exe",
    "taskmgr.exe",
];

pub fn is_protected_service(name: &str) -> bool {
    PROTECTED_SERVICES
        .iter()
        .any(|&s| s.eq_ignore_ascii_case(name))
}

pub fn is_protected_process(name: &str) -> bool {
    PROTECTED_PROCESSES
        .iter()
        .any(|&p| p.eq_ignore_ascii_case(name))
}

pub fn is_warned_process(name: &str) -> bool {
    WARNED_PROCESSES
        .iter()
        .any(|&p| p.eq_ignore_ascii_case(name))
}
