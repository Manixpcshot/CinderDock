use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;
use url::Url;

use crate::{
    error::{AppError, AppResult},
    models::{AppInfo, AuthMethod, Settings},
    security::{atomic_write, looks_like_secret},
};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub settings_path: PathBuf,
    pub database_path: PathBuf,
    pub secrets_path: PathBuf,
    pub captures_dir: PathBuf,
}

impl AppPaths {
    pub fn discover() -> AppResult<Self> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("CinderDock");
        Self::from_config_dir(config_dir)
    }

    pub fn from_config_dir(config_dir: PathBuf) -> AppResult<Self> {
        let data_dir = config_dir.join("data");
        let captures_dir = data_dir.join("captures");
        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(&captures_dir)?;
        fs::create_dir_all(config_dir.join("secure"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let secure_dir = config_dir.join("secure");
            fs::set_permissions(&secure_dir, fs::Permissions::from_mode(0o700))?;
        }
        Ok(Self {
            settings_path: config_dir.join("settings.json"),
            database_path: data_dir.join("cinderdock.sqlite3"),
            secrets_path: config_dir.join("secure").join("credentials.bin"),
            captures_dir,
            config_dir,
            data_dir,
        })
    }

    pub fn for_temp(path: &Path) -> AppResult<Self> {
        Self::from_config_dir(path.to_path_buf())
    }

    pub fn app_info(&self) -> AppInfo {
        AppInfo {
            version: env!("CARGO_PKG_VERSION").into(),
            platform: std::env::consts::OS.into(),
            config_dir: self.config_dir.display().to_string(),
            data_dir: self.data_dir.display().to_string(),
            #[cfg(target_os = "windows")]
            secret_backend: "Windows DPAPI (per-user)".into(),
            #[cfg(not(target_os = "windows"))]
            secret_backend: "In-memory development fallback (no plaintext persistence)".into(),
            update_check_enabled: false,
        }
    }
}

impl Settings {
    pub fn load_or_create(paths: &AppPaths) -> AppResult<Self> {
        if !paths.settings_path.exists() {
            let settings = Self::default();
            settings.save(paths)?;
            return Ok(settings);
        }
        let raw = fs::read_to_string(&paths.settings_path)?;
        match serde_json::from_str::<Self>(&raw) {
            Ok(settings) => {
                settings.validate()?;
                Ok(settings)
            }
            Err(error) => {
                let backup = paths.settings_path.with_extension("json.corrupt");
                let _ = fs::copy(&paths.settings_path, &backup);
                let settings = Self::default();
                settings.save(paths)?;
                tracing::warn!(
                    "settings.json was invalid and was backed up; defaults restored: {}",
                    error
                );
                Ok(settings)
            }
        }
    }

    pub fn save(&self, paths: &AppPaths) -> AppResult<()> {
        self.validate()?;
        let mut value = serde_json::to_value(self)?;
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "$schema".into(),
                Value::String("https://cinderdock.app/schemas/settings-v1.json".into()),
            );
        }
        atomic_write(&paths.settings_path, &serde_json::to_string_pretty(&value)?)?;
        Ok(())
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.schema_version != 1 {
            return Err(AppError::Validation(format!(
                "نسخهٔ طرح تنظیمات پشتیبانی نمیشود: {}",
                self.schema_version
            )));
        }
        if !(0.85..=1.35).contains(&self.font_scale) {
            return Err(AppError::Validation(
                "مقیاس فونت باید بین ۰٫۸۵ و ۱٫۳۵ باشد.".into(),
            ));
        }
        if !(1..=16).contains(&self.max_tool_iterations) || self.max_retries > 5 {
            return Err(AppError::Validation(
                "سقف حلقه ابزار یا تلاش دوباره خارج از محدوده است.".into(),
            ));
        }
        if !["ember-dark"].contains(&self.theme.as_str())
            || !["fa", "en"].contains(&self.ui_language.as_str())
            || !["native", "finglish", "latin"].contains(&self.text_mode.as_str())
        {
            return Err(AppError::Validation(
                "یکی از گزینههای ظاهر، زبان یا حالت نوشتار نامعتبر است.".into(),
            ));
        }
        for provider in &self.providers {
            let url = Url::parse(&provider.base_url)?;
            if !matches!(url.scheme(), "http" | "https") {
                return Err(AppError::Validation(
                    "آدرس پروایدر باید با http یا https شروع شود.".into(),
                ));
            }
            if !url.username().is_empty() || url.password().is_some() {
                return Err(AppError::Safety(
                    "اطلاعات احراز هویت نباید در Base URL قرار گیرد.".into(),
                ));
            }
            if provider.auth.method == AuthMethod::CustomHeader
                && provider
                    .auth
                    .header_name
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty()
            {
                return Err(AppError::Validation(
                    "نام هدر سفارشی احراز هویت لازم است.".into(),
                ));
            }
            for (name, value) in &provider.extra_headers {
                if looks_like_secret(name) || looks_like_secret(value) {
                    return Err(AppError::Safety(format!(
                        "هدر اضافی «{name}» ممکن است حاوی اطلاعات محرمانه باشد. کلید را در فیلد امن API Key وارد کنید."
                    )));
                }
            }
        }
        for model in &self.models {
            if !self.providers.iter().any(|p| p.id == model.provider_id) {
                return Err(AppError::Validation(format!(
                    "مدل «{}» به پروایدر ناموجودی اشاره میکند.",
                    model.display_name
                )));
            }
        }
        if let Some(default_id) = &self.default_model_id {
            if !self.models.iter().any(|m| &m.id == default_id) {
                return Err(AppError::Validation(
                    "مدل پیشفرض به مدل ناموجودی اشاره میکند.".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn sanitize(&mut self) {
        self.recent_projects.truncate(8);
        self.providers
            .sort_by(|a, b| a.display_name.cmp(&b.display_name));
        self.models
            .sort_by(|a, b| a.display_name.cmp(&b.display_name));
    }
}
