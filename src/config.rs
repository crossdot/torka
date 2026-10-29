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
    pub cache_dir: String,
    pub session_dir: String,
    pub cache_max_bytes: u64,
    pub max_active_sessions: usize,
    pub session_idle_timeout_secs: u64,
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
            cache_dir: env_str("CACHE_DIR", "/cache"),
            session_dir: env_str("SESSION_DIR", "/cache/sessions"),
            cache_max_bytes: env_parse("CACHE_MAX_BYTES", 2_147_483_648u64)?,
            max_active_sessions: env_parse("MAX_ACTIVE_SESSIONS", 2usize)?,
            session_idle_timeout_secs: env_parse("SESSION_IDLE_TIMEOUT_SECS", 900u64)?,
        })
    }

    pub fn bind_addr(&self) -> Result<std::net::SocketAddr, String> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|e| format!("invalid bind address {}:{}: {e}", self.host, self.port))
    }
}
