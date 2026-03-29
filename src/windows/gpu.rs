use crate::core::errors::{AppError, AppResult};
use crate::core::model::GpuPreference;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
    RegSetValueExW, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, HKEY,
};

const GPU_PREFS_KEY: &str = r"SOFTWARE\Microsoft\DirectX\UserGpuPreferences";

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn gpu_pref_value(pref: &GpuPreference) -> Option<String> {
    match pref {
        GpuPreference::Default => None,
        GpuPreference::PowerSaving => Some("GpuPreference=1;".to_string()),
        GpuPreference::HighPerformance => Some("GpuPreference=2;".to_string()),
    }
}

/// Read current GPU preference registry value for an exe filename.
pub fn read_gpu_preference(exe_filename: &str) -> Option<String> {
    unsafe {
        let key_w = to_wide(GPU_PREFS_KEY);
        let mut hkey: HKEY = 0;

        let r = RegOpenKeyExW(HKEY_CURRENT_USER, key_w.as_ptr(), 0, KEY_READ, &mut hkey);
        if r != 0 {
            return None;
        }

        let val_w = to_wide(exe_filename);
        let mut data = vec![0u8; 512];
        let mut data_len = data.len() as u32;
        let mut reg_type: u32 = 0;

        let result = RegQueryValueExW(
            hkey,
            val_w.as_ptr(),
            std::ptr::null_mut(),
            &mut reg_type,
            data.as_mut_ptr(),
            &mut data_len,
        );

        RegCloseKey(hkey);

        if result == 0 {
            let words: Vec<u16> = data[..data_len as usize]
                .chunks(2)
                .map(|c| u16::from_le_bytes([c[0], *c.get(1).unwrap_or(&0)]))
                .collect();
            let s = String::from_utf16_lossy(&words);
            Some(s.trim_end_matches('\0').to_string())
        } else {
            None
        }
    }
}

pub fn set_gpu_preference(
    exe_filename: &str,
    pref: &GpuPreference,
) -> AppResult<Option<String>> {
    // Read existing value before modifying (for restore)
    let previous = read_gpu_preference(exe_filename);

    let new_value = match gpu_pref_value(pref) {
        Some(v) => v,
        None => {
            // Default = no change, just return previous
            return Ok(previous);
        }
    };

    unsafe {
        let key_w = to_wide(GPU_PREFS_KEY);
        let mut hkey: HKEY = 0;
        let mut disposition: u32 = 0;

        let r = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_w.as_ptr(),
            0,
            std::ptr::null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            std::ptr::null(),
            &mut hkey,
            &mut disposition,
        );
        if r != 0 {
            return Err(AppError::RegistryError(format!(
                "RegCreateKeyEx failed: {}",
                r
            )));
        }

        let val_w = to_wide(exe_filename);
        let data_w: Vec<u16> = new_value
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let data_bytes: Vec<u8> = data_w.iter().flat_map(|w| w.to_le_bytes()).collect();

        let r2 = RegSetValueExW(
            hkey,
            val_w.as_ptr(),
            0,
            REG_SZ,
            data_bytes.as_ptr(),
            data_bytes.len() as u32,
        );
        RegCloseKey(hkey);

        if r2 != 0 {
            return Err(AppError::RegistryError(format!(
                "RegSetValueEx failed: {}",
                r2
            )));
        }
    }

    tracing::info!(
        "[INFO] GPU preference set for '{}': {:?}",
        exe_filename,
        pref
    );
    crate::logging::logger::add_to_buffer(
        "INFO",
        &format!("GPU preference {:?} set for '{}'", pref, exe_filename),
    );
    Ok(previous)
}

pub fn restore_gpu_preference(exe_filename: &str, previous: Option<&str>) -> AppResult<()> {
    unsafe {
        let key_w = to_wide(GPU_PREFS_KEY);
        let mut hkey: HKEY = 0;

        let r = RegOpenKeyExW(HKEY_CURRENT_USER, key_w.as_ptr(), 0, KEY_WRITE, &mut hkey);
        if r != 0 {
            return Ok(()); // Key doesn't exist, nothing to restore
        }

        let val_w = to_wide(exe_filename);

        match previous {
            None => {
                // Did not exist before — delete the value
                RegDeleteValueW(hkey, val_w.as_ptr());
            }
            Some(prev_val) => {
                // Existed — restore original value
                let data_w: Vec<u16> = prev_val
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let data_bytes: Vec<u8> = data_w.iter().flat_map(|w| w.to_le_bytes()).collect();
                RegSetValueExW(
                    hkey,
                    val_w.as_ptr(),
                    0,
                    REG_SZ,
                    data_bytes.as_ptr(),
                    data_bytes.len() as u32,
                );
            }
        }

        RegCloseKey(hkey);
    }

    tracing::info!("[INFO] GPU preference restored for '{}'", exe_filename);
    crate::logging::logger::add_to_buffer(
        "INFO",
        &format!("GPU preference restored for '{}'", exe_filename),
    );
    Ok(())
}
