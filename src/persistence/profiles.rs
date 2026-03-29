use crate::core::errors::{AppError, AppResult};
use crate::core::model::Profile;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub fn profiles_dir() -> PathBuf {
    PathBuf::from("profiles")
}

pub fn load_all_profiles() -> Vec<Profile> {
    let dir = profiles_dir();
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
        return Vec::new();
    }

    let mut profiles = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                match load_profile_from_path(&path) {
                    Ok(p) => profiles.push(p),
                    Err(e) => {
                        tracing::warn!("Failed to load profile {:?}: {}", path, e);
                    }
                }
            }
        }
    }
    profiles.sort_by(|a, b| a.friendly_name.cmp(&b.friendly_name));
    profiles
}

pub fn load_profile_from_path(path: &Path) -> AppResult<Profile> {
    let content = std::fs::read_to_string(path).map_err(AppError::Io)?;
    let profile: Profile = serde_json::from_str(&content).map_err(AppError::Json)?;
    Ok(profile)
}

pub fn save_profile(profile: &Profile) -> AppResult<()> {
    let dir = profiles_dir();
    std::fs::create_dir_all(&dir).map_err(AppError::Io)?;

    let filename = format!("{}.json", profile.name.replace(' ', "-").to_lowercase());
    let path = dir.join(&filename);

    let content = serde_json::to_string_pretty(profile).map_err(AppError::Json)?;
    std::fs::write(&path, content).map_err(AppError::Io)?;
    tracing::info!("Profile '{}' saved to {:?}", profile.name, path);
    Ok(())
}

pub fn delete_profile(profile: &Profile) -> AppResult<()> {
    let dir = profiles_dir();
    let filename = format!("{}.json", profile.name.replace(' ', "-").to_lowercase());
    let path = dir.join(&filename);

    if path.exists() {
        std::fs::remove_file(&path).map_err(AppError::Io)?;
        tracing::info!("Profile '{}' deleted", profile.name);
    }
    Ok(())
}

pub fn export_profile(profile: &Profile, path: &Path) -> AppResult<()> {
    let content = serde_json::to_string_pretty(profile).map_err(AppError::Json)?;
    std::fs::write(path, content).map_err(AppError::Io)?;
    Ok(())
}

pub fn import_profile(path: &Path) -> AppResult<Profile> {
    let mut profile = load_profile_from_path(path)?;
    // Assign new UUID to avoid collisions
    profile.id = Uuid::new_v4();
    profile.created_at = chrono::Utc::now();
    profile.updated_at = chrono::Utc::now();
    Ok(profile)
}
