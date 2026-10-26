use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use regex::Regex;
use sha2::{Digest, Sha256};
use url::Url;

use crate::error::{AppError, AppResult};

pub const MAX_FILE_BYTES: usize = 1024 * 1024;
pub const MAX_OUTPUT_CHARS: usize = 96_000;
pub const MAX_WEB_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;

const SENSITIVE_NAMES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    ".git",
    ".ssh",
    ".aws",
    ".kube",
    ".docker",
    ".npmrc",
    ".netrc",
    ".htpasswd",
    ".pypirc",
    "credentials",
    "credentials.json",
    "secrets",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    "keystore",
    "service-account.json",
];

const SKIP_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".nuxt",
    ".venv",
    "venv",
    "__pycache__",
    ".cinderdock",
];

pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4().simple())
}

pub fn canonical_project_root(path: &Path) -> AppResult<PathBuf> {
    let root = fs::canonicalize(path).map_err(|e| {
        AppError::Validation(format!("پوشه پروژه معتبر نیست: {} ({e})", path.display()))
    })?;
    if !root.is_dir() {
        return Err(AppError::Validation("مسیر انتخابشده پوشه نیست.".into()));
    }
    Ok(root)
}

pub fn project_relative_path(root: &Path, relative: &str) -> AppResult<PathBuf> {
    if relative.trim().is_empty() || relative.contains('\0') {
        return Err(AppError::Safety("مسیر فایل معتبر نیست.".into()));
    }
    let candidate = Path::new(relative);
    for part in candidate.components() {
        match part {
            Component::Normal(_) | Component::CurDir => {}
            _ => {
                return Err(AppError::Safety(
                    "مسیر مطلق، .. یا پیوند ویندوزی مجاز نیست.".into(),
                ))
            }
        }
    }
    if is_sensitive_path(candidate) {
        return Err(AppError::Safety(
            "دسترسی به این مسیر حساس بهطور پیشفرض مسدود است.".into(),
        ));
    }

    let root = canonical_project_root(root)?;
    let parent_rel = candidate.parent().unwrap_or_else(|| Path::new(""));
    let parent = root.join(parent_rel);
    let canonical_parent = fs::canonicalize(&parent).map_err(|e| {
        AppError::Safety(format!(
            "پوشه مقصد قابل تأیید نیست؛ از پیوندهای نمادین خارج از پروژه جلوگیری شد. ({e})"
        ))
    })?;
    if !canonical_parent.starts_with(&root) {
        return Err(AppError::Safety(
            "خروج از پوشه پروژه از طریق پیوند نمادین مسدود شد.".into(),
        ));
    }
    let file_name = candidate
        .file_name()
        .ok_or_else(|| AppError::Safety("انتهای مسیر باید نام یک فایل باشد.".into()))?;
    let resolved = canonical_parent.join(file_name);
    if let Ok(existing) = fs::canonicalize(&resolved) {
        if !existing.starts_with(&root) {
            return Err(AppError::Safety(
                "فایل مقصد به بیرون از پروژه اشاره میکند.".into(),
            ));
        }
    }
    Ok(resolved)
}

pub fn is_sensitive_path(path: &Path) -> bool {
    let lower = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    for name in SENSITIVE_NAMES {
        if lower.split('/').any(|part| part == *name) {
            return true;
        }
    }
    lower.ends_with(".pem")
        || lower.ends_with(".key")
        || lower.ends_with(".pfx")
        || lower.ends_with(".p12")
        || lower.ends_with(".keystore")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("access_token")
        || lower.contains("refresh_token")
}

pub fn should_skip_dir(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    SKIP_DIRS.contains(&lowered.as_str()) || is_sensitive_path(Path::new(&lowered))
}

pub fn safe_read_text(path: &Path, max_bytes: usize) -> AppResult<String> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_bytes as u64 {
        return Err(AppError::Safety(format!(
            "اندازه فایل از سقف {max_bytes} بایت بیشتر است."
        )));
    }
    let bytes = fs::read(path)?;
    if bytes.contains(&0) {
        return Err(AppError::Safety(
            "فایل باینری است و بهعنوان متن خوانده نمیشود.".into(),
        ));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn atomic_write(path: &Path, contents: &str) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension(format!(
        "{}.cinderdock-tmp",
        path.extension().and_then(|x| x.to_str()).unwrap_or("dat")
    ));
    fs::write(&temp, contents.as_bytes())?;
    fs::rename(&temp, path)?;
    Ok(())
}

pub fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn truncate_chars(text: &str, max: usize) -> (String, bool) {
    let mut out = String::new();
    let mut count = 0usize;
    for ch in text.chars() {
        if count >= max {
            return (
                format!("{out}\n… [CinderDock: output truncated at {max} characters]"),
                true,
            );
        }
        out.push(ch);
        count += 1;
    }
    (out, false)
}

pub fn redact_text(secrets: &[String], input: &str) -> String {
    let mut output = input.to_string();
    for secret in secrets {
        let trimmed = secret.trim();
        if trimmed.len() >= 6 {
            output = output.replace(trimmed, "[REDACTED]");
        }
    }
    const PATTERNS: &[(&str, &str)] = &[
        (
            r"(?i)(authorization\s*[:=]\s*bearer\s+)[A-Za-z0-9._\-]+",
            "$1[REDACTED]",
        ),
        (r#"(?i)(x-api-key\s*[:=]\s*)[^\s,;}\"']+"#, "$1[REDACTED]"),
        (
            r#"(?i)(api[_-]?key\s*[:=]\s*)[^\s,;}&\"']+"#,
            "$1[REDACTED]",
        ),
        (r"(?i)(bearer\s+)[A-Za-z0-9._\-]{12,}", "$1[REDACTED]"),
        (r"sk-[A-Za-z0-9_\-]{16,}", "[REDACTED]"),
    ];
    for (pattern, replacement) in PATTERNS {
        if let Ok(re) = Regex::new(*pattern) {
            output = re.replace_all(&output, *replacement).into_owned();
        }
    }
    output
}

pub fn validate_public_http_url(raw: &str) -> AppResult<Url> {
    let url = Url::parse(raw)?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::Safety(
            "فقط پروتکلهای HTTP و HTTPS پشتیبانی میشوند.".into(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::Safety(
            "قراردادن نام کاربری یا رمز در URL مجاز نیست.".into(),
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| AppError::Safety("URL فاقد میزبان معتبر است.".into()))?;
    if is_private_host(host) {
        return Err(AppError::Safety(
            "دسترسی وب به میزبانهای محلی یا خصوصی مسدود است.".into(),
        ));
    }
    Ok(url)
}

pub fn is_private_host(host: &str) -> bool {
    let normalized = host
        .trim_matches(|c| c == '[' || c == ']')
        .to_ascii_lowercase();
    if normalized == "localhost"
        || normalized.ends_with(".localhost")
        || normalized.ends_with(".local")
    {
        return true;
    }
    if let Ok(ip) = normalized.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v4) => {
                v4.is_private()
                    || v4.is_loopback()
                    || v4.is_link_local()
                    || v4.is_broadcast()
                    || v4.is_unspecified()
            }
            std::net::IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
        };
    }
    false
}

pub fn looks_like_secret(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("bearer ")
        || lower.starts_with("sk-")
        || lower.starts_with("api-key ")
        || lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("api_key")
        || lower.contains("apikey")
}

pub fn command_risk(command: &str) -> &'static str {
    let lower = command.to_ascii_lowercase();
    if lower.contains("rm -rf")
        || lower.contains("del /f")
        || lower.contains("format ")
        || lower.contains("rd /s")
        || lower.contains("remove-item -recurse")
        || lower.contains("git reset --hard")
        || lower.contains("git clean -fd")
        || lower.contains("shutdown")
    {
        "critical"
    } else if lower.contains("git push")
        || lower.contains("npm publish")
        || lower.contains("cargo publish")
        || lower.contains("curl ")
        || lower.contains("iwr ")
    {
        "high"
    } else {
        "normal"
    }
}

pub fn split_terminal_lines(stdout: &str, stderr: &str) -> (Vec<String>, Vec<String>) {
    fn cap(text: &str) -> Vec<String> {
        let (truncated, _) = truncate_chars(text, MAX_OUTPUT_CHARS);
        truncated
            .lines()
            .take(400)
            .map(|line| line.to_string())
            .collect()
    }
    (cap(stdout), cap(stderr))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[test]
    fn rejects_parent_traversal_and_absolute_paths() {
        let root = tempfile::tempdir().unwrap();
        assert!(project_relative_path(root.path(), "../secret.txt").is_err());
        assert!(project_relative_path(root.path(), "/tmp/x.txt").is_err());
        assert!(project_relative_path(root.path(), "src/../.env").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn blocks_symlink_escape() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("link")).unwrap();
        fs::remove_dir(root.path().join("link")).unwrap();
        symlink(outside.path(), root.path().join("link")).unwrap();
        assert!(project_relative_path(root.path(), "link/new.txt").is_err());
    }

    #[test]
    fn redacts_known_secrets_and_key_shapes() {
        let secret = format!("{}-{}", "sk", "super-secret-value-123456");
        let redacted = redact_text(
            &[secret.clone()],
            &format!(
                "Authorization: Bearer {secret} and {}-abcdefghijklmnopqrst",
                "sk"
            ),
        );
        assert!(!redacted.contains(&secret));
        assert!(!redacted.contains(&format!("{}-abcdefghijklmnopqrst", "sk")));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn blocks_private_web_hosts() {
        assert!(validate_public_http_url("http://127.0.0.1:3000").is_err());
        assert!(validate_public_http_url("http://localhost/admin").is_err());
        assert!(validate_public_http_url("http://user:pass@example.com").is_err());
        assert!(validate_public_http_url("https://example.com/a").is_ok());
    }

    #[test]
    fn truncates_by_unicode_chars() {
        let (text, did) = truncate_chars("سلام", 2);
        assert!(did);
        assert!(text.starts_with("سل"));
    }
}
