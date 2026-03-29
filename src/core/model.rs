use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessPriority {
    Idle,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
    Realtime,
}

impl Default for ProcessPriority {
    fn default() -> Self {
        ProcessPriority::High
    }
}

impl std::fmt::Display for ProcessPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessPriority::Idle => write!(f, "Idle"),
            ProcessPriority::BelowNormal => write!(f, "Below Normal"),
            ProcessPriority::Normal => write!(f, "Normal"),
            ProcessPriority::AboveNormal => write!(f, "Above Normal"),
            ProcessPriority::High => write!(f, "High"),
            ProcessPriority::Realtime => write!(f, "Realtime (DANGEROUS)"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuAffinity {
    pub mask: Option<u64>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GpuPreference {
    Default,
    PowerSaving,
    HighPerformance,
}

impl Default for GpuPreference {
    fn default() -> Self {
        GpuPreference::HighPerformance
    }
}

impl std::fmt::Display for GpuPreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuPreference::Default => write!(f, "Default"),
            GpuPreference::PowerSaving => write!(f, "Power Saving"),
            GpuPreference::HighPerformance => write!(f, "High Performance"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAction {
    pub service_name: String,
    pub display_name: String,
    #[serde(default = "default_true")]
    pub stop_on_launch: bool,
    #[serde(default)]
    pub was_running: Option<bool>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessAction {
    pub exe_name: String,
    pub display_name: String,
    #[serde(default = "default_true")]
    pub kill_on_launch: bool,
    #[serde(default)]
    pub is_ignored: bool,
    #[serde(default)]
    pub relaunch_after: bool,
    #[serde(default)]
    pub captured_cmdline: Option<String>,
    #[serde(default)]
    pub captured_working_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskAction {
    pub task_path: String,
    pub display_name: String,
    #[serde(default = "default_true")]
    pub disable_on_launch: bool,
    #[serde(default)]
    pub was_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryOptimization {
    #[serde(default)]
    pub trim_other_processes: bool,
    #[serde(default)]
    pub target_min_working_set_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayConfig {
    #[serde(default = "default_after_services")]
    pub after_services_ms: u64,
    #[serde(default = "default_after_processes")]
    pub after_processes_ms: u64,
    #[serde(default = "default_after_launch")]
    pub after_launch_ms: u64,
    /// Seconds to wait after the launched PID exits before checking for a
    /// re-launched successor process (e.g. DCS launcher → actual game).
    /// Increase this if the game takes a long time to spawn after the launcher dies.
    #[serde(default = "default_relaunch_grace_secs")]
    pub relaunch_grace_secs: u64,
}

fn default_after_services() -> u64 { 1000 }
fn default_after_processes() -> u64 { 500 }
fn default_after_launch() -> u64 { 3000 }
fn default_relaunch_grace_secs() -> u64 { 30 }

impl Default for DelayConfig {
    fn default() -> Self {
        DelayConfig {
            after_services_ms: 1000,
            after_processes_ms: 500,
            after_launch_ms: 3000,
            relaunch_grace_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub name: String,
    pub friendly_name: String,
    pub executable_path: String,
    #[serde(default)]
    pub launch_args: Option<String>,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub services: Vec<ServiceAction>,
    #[serde(default)]
    pub processes: Vec<ProcessAction>,
    #[serde(default)]
    pub scheduled_tasks: Vec<ScheduledTaskAction>,
    #[serde(default)]
    pub process_priority: ProcessPriority,
    #[serde(default)]
    pub cpu_affinity: CpuAffinity,
    #[serde(default)]
    pub gpu_preference: GpuPreference,
    #[serde(default)]
    pub memory_optimization: MemoryOptimization,
    #[serde(default)]
    pub delays: DelayConfig,
    #[serde(default)]
    pub notes: String,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub icon_path: Option<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            id: Uuid::new_v4(),
            name: String::new(),
            friendly_name: String::new(),
            executable_path: String::new(),
            launch_args: None,
            working_directory: None,
            services: Vec::new(),
            processes: Vec::new(),
            scheduled_tasks: Vec::new(),
            process_priority: ProcessPriority::High,
            cpu_affinity: CpuAffinity::default(),
            gpu_preference: GpuPreference::HighPerformance,
            memory_optimization: MemoryOptimization::default(),
            delays: DelayConfig::default(),
            notes: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            icon_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionPhase {
    Idle,
    PreLaunch,
    Running,
    Restoring,
    Completed,
    Failed,
    Interrupted,
}

impl Default for SessionPhase {
    fn default() -> Self {
        SessionPhase::Idle
    }
}

impl std::fmt::Display for SessionPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionPhase::Idle => write!(f, "Idle"),
            SessionPhase::PreLaunch => write!(f, "Pre-Launch"),
            SessionPhase::Running => write!(f, "Running"),
            SessionPhase::Restoring => write!(f, "Restoring"),
            SessionPhase::Completed => write!(f, "Completed"),
            SessionPhase::Failed => write!(f, "Failed"),
            SessionPhase::Interrupted => write!(f, "Interrupted"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KilledProcessRecord {
    pub exe_name: String,
    pub pid: u32,
    pub cmdline: Option<String>,
    pub working_dir: Option<String>,
    pub should_relaunch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemSnapshot {
    pub captured_at: DateTime<Utc>,
    pub services: HashMap<String, bool>,
    pub scheduled_tasks: HashMap<String, bool>,
    pub killed_processes: Vec<KilledProcessRecord>,
    pub gpu_preference_before: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionState {
    pub profile_id: Option<Uuid>,
    pub profile_name: Option<String>,
    pub target_pid: Option<u32>,
    pub phase: SessionPhase,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub pre_session_snapshot: Option<SystemSnapshot>,
    pub session_log: Vec<SessionLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_profiles_dir")]
    pub profiles_dir: String,
    #[serde(default = "default_logs_dir")]
    pub logs_dir: String,
    #[serde(default = "default_true")]
    pub confirm_realtime_priority: bool,
    #[serde(default = "default_true")]
    pub confirm_before_launch: bool,
}

fn default_profiles_dir() -> String { "profiles".to_string() }
fn default_logs_dir() -> String { "logs".to_string() }

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            profiles_dir: "profiles".to_string(),
            logs_dir: "logs".to_string(),
            confirm_realtime_priority: true,
            confirm_before_launch: true,
        }
    }
}
