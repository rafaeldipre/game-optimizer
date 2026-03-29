use crate::core::model::{AppConfig, Profile, SessionPhase, SessionState};
use crate::persistence::{config, profiles as profiles_persistence, session as session_persistence};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub profiles: Vec<Profile>,
    pub selected_profile_index: Option<usize>,
    /// Shared with the optimizer task — both read/write through this.
    pub shared_session: Arc<Mutex<SessionState>>,
    pub config: AppConfig,
    pub is_elevated: bool,
    pub stop_signal: Arc<AtomicBool>,
    pub interrupted_session: Option<SessionState>,
    // UI state
    pub show_realtime_warning: bool,
    pub show_recovery_dialog: bool,
    pub active_tab: Tab,
    pub profile_edit_state: Option<Profile>,
    pub is_editing_profile: bool,
    pub new_service_input: String,
    pub new_process_input: String,
    // Process picker dialog state
    pub show_process_picker: bool,
    pub process_picker_list: Vec<(String, u32)>, // (exe_name, pid) deduplicated
    pub process_picker_filter: String,
    // Pre-launch program editor state (inline in profile editor)
    pub new_prelaunch_exe: String,
    pub new_prelaunch_name: String,
    pub new_prelaunch_args: String,
    pub new_prelaunch_wait_ms: u64,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Tab {
    Dashboard,
    Profiles,
    Logs,
    Settings,
}

impl AppState {
    pub fn new() -> Self {
        let profiles = profiles_persistence::load_all_profiles();
        let config = config::load_config();

        #[cfg(target_os = "windows")]
        let is_elevated = crate::windows::elevation::is_elevated();
        #[cfg(not(target_os = "windows"))]
        let is_elevated = false;

        let interrupted_session = session_persistence::load_session().and_then(|s| {
            match s.phase {
                SessionPhase::Running | SessionPhase::PreLaunch | SessionPhase::Restoring => Some(s),
                _ => None,
            }
        });

        let show_recovery_dialog = interrupted_session.is_some();

        AppState {
            profiles,
            selected_profile_index: None,
            shared_session: Arc::new(Mutex::new(SessionState::default())),
            config,
            is_elevated,
            stop_signal: Arc::new(AtomicBool::new(false)),
            interrupted_session,
            show_realtime_warning: false,
            show_recovery_dialog,
            active_tab: Tab::Dashboard,
            profile_edit_state: None,
            is_editing_profile: false,
            new_service_input: String::new(),
            new_process_input: String::new(),
            show_process_picker: false,
            process_picker_list: Vec::new(),
            process_picker_filter: String::new(),
            new_prelaunch_exe: String::new(),
            new_prelaunch_name: String::new(),
            new_prelaunch_args: String::new(),
            new_prelaunch_wait_ms: 2000,
        }
    }

    pub fn reload_profiles(&mut self) {
        self.profiles = profiles_persistence::load_all_profiles();
    }

    pub fn stop_session(&self) {
        self.stop_signal.store(true, Ordering::Relaxed);
    }

    pub fn reset_stop_signal(&mut self) {
        self.stop_signal = Arc::new(AtomicBool::new(false));
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
