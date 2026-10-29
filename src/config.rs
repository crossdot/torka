//! Environment-driven configuration with validation.
//!
//! Every setting mirrors `ENV.example` and the required settings table in
//! `DOCKER_SYNOLOGY.md`. Unknown fields are not inferred; absent values fall
//! back to documented defaults.

use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwAccelBackend {
    /// Probe available hardware backends, falling back to software.
    Auto,
    /// CPU-only transcoding.
    Software,
    /// Intel iGPU and AMD GPUs via the Linux VA-API.
    Vaapi,
    /// NVIDIA GPUs via NVENC/NVDEC.
    Nvenc,
}

impl HwAccelBackend {
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "auto" => Ok(Self::Auto),
            "software" => Ok(Self::Software),
            "vaapi" => Ok(Self::Vaapi),
            "nvenc" => Ok(Self::Nvenc),
            other => Err(format!(
                "invalid HWACCEL_MODE={other:?}: expected auto|software|vaapi|nvenc"
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Software => "software",
            Self::Vaapi => "vaapi",
            Self::Nvenc => "nvenc",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub rust_log: String,
    /// `None` when unset or still the `change-me-before-use` placeholder.
    pub api_token: Option<String>,
    pub hwaccel_backend: HwAccelBackend,
    /// Optional device node for the selected backend (e.g. `/dev/dri/renderD128`
    /// for VA-API). `None` = auto-detect.
    pub hwaccel_device: Option<String>,
    pub hwaccel_required: bool,
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub ffmpeg_start_timeout_secs: u64,
    pub torrent_resolve_timeout_secs: u64,
    pub cache_dir: String,
    pub session_dir: String,
    pub static_dir: String,
    pub cache_max_bytes: u64,
    pub max_active_sessions: usize,
    pub session_idle_timeout_secs: u64,
    pub search_timeout_secs: u64,
    pub search_max_parallel: usize,
    pub search_cache_ttl_secs: u64,
    pub search_cache_max_entries: usize,
    pub provider_fake_enabled: bool,
    pub provider_rutracker_enabled: bool,
    pub provider_rutor_enabled: bool,
    pub provider_rutracker_forum_id: String,
    pub provider_rutracker_urls: Vec<String>,
    pub provider_rutor_urls: Vec<String>,
}

fn env_str(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}

fn env_parse<T>(key: &str, default: T) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(raw) => raw
            .parse::<T>()
            .map_err(|e| format!("invalid {key}={raw:?}: {e}")),
        Err(_) => Ok(default),
    }
}

/// Comma-separated list, trimmed and with trailing slashes stripped.
fn env_list(key: &str, default: &str) -> Vec<String> {
    env::var(key)
        .unwrap_or_else(|_| default.to_owned())
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches('/').to_owned())
        .collect()
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let hwaccel_backend = HwAccelBackend::parse(&env_str("HWACCEL_MODE", "auto"))?;

        let api_token = match env::var("APP_API_TOKEN") {
            Ok(t) if !t.is_empty() && t != "change-me-before-use" => Some(t),
            _ => None,
        };

        Ok(Self {
            host: env_str("APP_HOST", "0.0.0.0"),
            port: env_parse("APP_PORT", 8080u16)?,
            rust_log: env_str("RUST_LOG", "info"),
            api_token,
            hwaccel_backend,
            hwaccel_device: env::var("HWACCEL_DEVICE").ok().filter(|v| !v.is_empty()),
            hwaccel_required: env_parse("HWACCEL_REQUIRED", false)?,
            ffmpeg_path: env_str("FFMPEG_PATH", "ffmpeg"),
            ffprobe_path: env_str("FFPROBE_PATH", "ffprobe"),
            ffmpeg_start_timeout_secs: env_parse("FFMPEG_START_TIMEOUT_SECS", 30u64)?,
            torrent_resolve_timeout_secs: env_parse("TORRENT_RESOLVE_TIMEOUT_SECS", 60u64)?,
            cache_dir: env_str("CACHE_DIR", "/cache"),
            session_dir: env_str("SESSION_DIR", "/cache/sessions"),
            static_dir: env_str("STATIC_DIR", "frontend/dist"),
            cache_max_bytes: env_parse("CACHE_MAX_BYTES", 2_147_483_648u64)?,
            max_active_sessions: env_parse("MAX_ACTIVE_SESSIONS", 2usize)?,
            session_idle_timeout_secs: env_parse("SESSION_IDLE_TIMEOUT_SECS", 900u64)?,
            search_timeout_secs: env_parse("SEARCH_TIMEOUT_SECS", 8u64)?,
            search_max_parallel: env_parse("SEARCH_MAX_PARALLEL", 4usize)?,
            search_cache_ttl_secs: env_parse("SEARCH_CACHE_TTL_SECS", 60u64)?,
            search_cache_max_entries: env_parse("SEARCH_CACHE_MAX_ENTRIES", 200usize)?,
            provider_fake_enabled: env_parse("PROVIDER_FAKE_ENABLED", false)?,
            provider_rutracker_enabled: env_parse("PROVIDER_RUTRACKER_ENABLED", false)?,
            provider_rutor_enabled: env_parse("PROVIDER_RUTOR_ENABLED", false)?,
            provider_rutracker_forum_id: env_str("PROVIDER_RUTRACKER_FORUM_ID", "0"),
            provider_rutracker_urls: env_list(
                "PROVIDER_RUTRACKER_URLS",
                "https://feed.rutracker.cc",
            ),
            provider_rutor_urls: env_list(
                "PROVIDER_RUTOR_URLS",
                "https://rutor.info,https://rutor.is",
            ),
        })
    }

    pub fn bind_addr(&self) -> Result<std::net::SocketAddr, String> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|e| format!("invalid bind address {}:{}: {e}", self.host, self.port))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 8080,
            rust_log: "info".into(),
            api_token: None,
            hwaccel_backend: HwAccelBackend::Auto,
            hwaccel_device: None,
            hwaccel_required: false,
            ffmpeg_path: "ffmpeg".into(),
            ffprobe_path: "ffprobe".into(),
            ffmpeg_start_timeout_secs: 30,
            torrent_resolve_timeout_secs: 60,
            cache_dir: "/cache".into(),
            session_dir: "/cache/sessions".into(),
            static_dir: "frontend/dist".into(),
            cache_max_bytes: 2_147_483_648,
            max_active_sessions: 2,
            session_idle_timeout_secs: 900,
            search_timeout_secs: 8,
            search_max_parallel: 4,
            search_cache_ttl_secs: 60,
            search_cache_max_entries: 200,
            provider_fake_enabled: false,
            provider_rutracker_enabled: false,
            provider_rutor_enabled: false,
            provider_rutracker_forum_id: "0".into(),
            provider_rutracker_urls: vec!["https://feed.rutracker.cc".into()],
            provider_rutor_urls: vec!["https://rutor.info".into(), "https://rutor.is".into()],
        }
    }
}
