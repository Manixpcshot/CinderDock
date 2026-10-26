use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[cfg(target_os = "windows")]
use base64::Engine;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use std::fs;

use crate::error::{AppError, AppResult};

#[derive(Debug, Default, Serialize, Deserialize)]
struct Envelope {
    version: u32,
    #[serde(default)]
    blobs: HashMap<String, String>,
}

#[derive(Debug)]
struct Inner {
    path: PathBuf,
    values: Mutex<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct SecretStore {
    inner: Arc<Inner>,
}

impl SecretStore {
    pub fn new(path: PathBuf) -> AppResult<Self> {
        let values = Self::load_from_disk(&path)?;
        Ok(Self {
            inner: Arc::new(Inner {
                path,
                values: Mutex::new(values),
            }),
        })
    }

    pub fn get(&self, provider_id: &str) -> Option<String> {
        self.inner
            .values
            .lock()
            .ok()
            .and_then(|values| values.get(provider_id).cloned())
    }

    pub fn has(&self, provider_id: &str) -> bool {
        self.get(provider_id)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    }

    pub fn set(&self, provider_id: &str, secret: &str) -> AppResult<()> {
        let secret = secret.trim();
        if secret.is_empty() {
            return Err(AppError::Validation("کلید API نمیتواند خالی باشد.".into()));
        }
        {
            let mut values = self
                .inner
                .values
                .lock()
                .map_err(|_| AppError::Internal("secret store lock poisoned".into()))?;
            values.insert(provider_id.to_string(), secret.to_string());
        }
        self.persist()
    }

    pub fn delete(&self, provider_id: &str) -> AppResult<()> {
        {
            let mut values = self
                .inner
                .values
                .lock()
                .map_err(|_| AppError::Internal("secret store lock poisoned".into()))?;
            values.remove(provider_id);
        }
        self.persist()
    }

    pub fn all(&self) -> Vec<String> {
        self.inner
            .values
            .lock()
            .ok()
            .map(|values| values.values().cloned().collect())
            .unwrap_or_default()
    }

    fn persist(&self) -> AppResult<()> {
        #[cfg(not(target_os = "windows"))]
        {
            // Development fallback: never write plaintext credentials outside Windows.
            return Ok(());
        }
        #[cfg(target_os = "windows")]
        {
            let snapshot: HashMap<String, String> = self
                .inner
                .values
                .lock()
                .map_err(|_| AppError::Internal("secret store lock poisoned".into()))?
                .clone();
            let mut blobs = HashMap::new();
            for (id, value) in snapshot {
                let encrypted = dpapi::protect(value.as_bytes())?;
                blobs.insert(
                    id,
                    base64::engine::general_purpose::STANDARD.encode(encrypted),
                );
            }
            let envelope = Envelope { version: 1, blobs };
            let bytes = serde_json::to_vec(&envelope)?;
            if let Some(parent) = self.inner.path.parent() {
                fs::create_dir_all(parent)?;
            }
            crate::security::atomic_write(&self.inner.path, &String::from_utf8_lossy(&bytes))?;
            Ok(())
        }
    }

    fn load_from_disk(path: &PathBuf) -> AppResult<HashMap<String, String>> {
        #[cfg(not(target_os = "windows"))]
        {
            let _ = path;
            return Ok(HashMap::new());
        }
        #[cfg(target_os = "windows")]
        {
            if !path.exists() {
                return Ok(HashMap::new());
            }
            let raw = fs::read_to_string(path)?;
            let envelope: Envelope = serde_json::from_str(&raw).unwrap_or(Envelope::default());
            let mut values = HashMap::new();
            for (id, encoded) in envelope.blobs {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(encoded.as_bytes())
                    .map_err(|e| AppError::Internal(format!("invalid credential blob: {e}")))?;
                let plain = dpapi::unprotect(&bytes)?;
                values.insert(id, String::from_utf8_lossy(&plain).into_owned());
            }
            Ok(values)
        }
    }
}

#[cfg(target_os = "windows")]
mod dpapi {
    use std::{ffi::c_void, io, ptr};

    #[repr(C)]
    struct DataBlob {
        cb_data: u32,
        pb_data: *mut u8,
    }

    #[link(name = "crypt32")]
    extern "system" {
        fn CryptProtectData(
            data_in: *const DataBlob,
            description: *const u16,
            entropy: *const DataBlob,
            reserved: *mut c_void,
            prompt: *mut c_void,
            flags: u32,
            data_out: *mut DataBlob,
        ) -> i32;
        fn CryptUnprotectData(
            data_in: *const DataBlob,
            description: *mut *mut u16,
            entropy: *const DataBlob,
            reserved: *mut c_void,
            prompt: *mut c_void,
            flags: u32,
            data_out: *mut DataBlob,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(handle: *mut c_void) -> *mut c_void;
    }

    const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

    fn wide_description() -> Vec<u16> {
        "CinderDock provider credential\0".encode_utf16().collect()
    }

    pub(super) fn protect(data: &[u8]) -> crate::error::AppResult<Vec<u8>> {
        let input = DataBlob {
            cb_data: data.len() as u32,
            pb_data: data.as_ptr() as *mut u8,
        };
        let mut output = DataBlob {
            cb_data: 0,
            pb_data: ptr::null_mut(),
        };
        let description = wide_description();
        let ok = unsafe {
            CryptProtectData(
                &input,
                description.as_ptr(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 || output.pb_data.is_null() {
            return Err(crate::error::AppError::Internal(format!(
                "CryptProtectData failed: {}",
                io::Error::last_os_error()
            )));
        }
        let result =
            unsafe { std::slice::from_raw_parts(output.pb_data, output.cb_data as usize).to_vec() };
        unsafe {
            LocalFree(output.pb_data as *mut c_void);
        }
        Ok(result)
    }

    pub(super) fn unprotect(data: &[u8]) -> crate::error::AppResult<Vec<u8>> {
        let input = DataBlob {
            cb_data: data.len() as u32,
            pb_data: data.as_ptr() as *mut u8,
        };
        let mut output = DataBlob {
            cb_data: 0,
            pb_data: ptr::null_mut(),
        };
        let ok = unsafe {
            CryptUnprotectData(
                &input,
                ptr::null_mut(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 || output.pb_data.is_null() {
            return Err(crate::error::AppError::Internal(format!(
                "CryptUnprotectData failed: {}",
                io::Error::last_os_error()
            )));
        }
        let result =
            unsafe { std::slice::from_raw_parts(output.pb_data, output.cb_data as usize).to_vec() };
        unsafe {
            LocalFree(output.pb_data as *mut c_void);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_secret_in_memory_without_plaintext_disk_on_non_windows() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.bin");
        let store = SecretStore::new(path.clone()).unwrap();
        store.set("provider", "sk-test-secret-123456").unwrap();
        assert_eq!(
            store.get("provider").as_deref(),
            Some("sk-test-secret-123456")
        );
        assert_eq!(store.all(), vec!["sk-test-secret-123456".to_string()]);
        store.delete("provider").unwrap();
        assert!(store.get("provider").is_none());
        #[cfg(not(target_os = "windows"))]
        assert!(!path.exists());
    }
}
