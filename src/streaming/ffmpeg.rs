//! FFmpeg HLS runner: spawns FFmpeg as a child process with validated args,
//! writing HLS into a session directory, with cancellation, a bounded stderr
//! tail for diagnostics, and graceful shutdown.
//!
//! Args are built from an internal config only — never from client input, and
//! never through a shell (STREAMING.md §12.2).

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tokio::io::AsyncReadExt;

use crate::error::AppError;

pub struct HlsConfig {
    pub ffmpeg_path: String,
    /// Loopback source URL (seekable HTTP range) supplied by the source server.
    pub input_url: String,
    pub output_dir: PathBuf,
    /// Seek start in seconds (input `-ss`); `None` = start from the beginning.
    pub start_time_secs: Option<f64>,
}

/// Build the FFmpeg argument vector from config. Pure for testability.
fn build_args(config: &HlsConfig) -> Vec<String> {
    let mut args = vec!["-v".to_string(), "error".to_string()];

    if let Some(ss) = config.start_time_secs {
        args.push("-ss".to_string());
        args.push(ss.to_string());
    }

    let playlist = config.output_dir.join("out.m3u8");
    let segments = config.output_dir.join("seg_%03d.ts");

    args.extend([
        "-i".to_string(),
        config.input_url.clone(),
        "-c".to_string(),
        "copy".to_string(),
        "-f".to_string(),
        "hls".to_string(),
        "-hls_time".to_string(),
        "2".to_string(),
        "-hls_list_size".to_string(),
        "0".to_string(),
        "-hls_base_url".to_string(),
        "segments/".to_string(),
        "-hls_segment_filename".to_string(),
        segments.to_string_lossy().into_owned(),
        playlist.to_string_lossy().into_owned(),
    ]);

    args
}

/// Bounded ring buffer holding the tail of FFmpeg's stderr (bounded diagnostics).
struct StderrTail {
    buf: VecDeque<u8>,
    cap: usize,
}

impl StderrTail {
    fn new(cap: usize) -> Self {
        Self {
            buf: VecDeque::new(),
            cap,
        }
    }

    fn push(&mut self, chunk: &[u8]) {
        self.buf.extend(chunk);
        while self.buf.len() > self.cap {
            self.buf.pop_front();
        }
    }

    fn as_string(&self) -> String {
        String::from_utf8_lossy(&self.buf.iter().copied().collect::<Vec<_>>()).into_owned()
    }
}

pub struct FfmpegRunner {
    child: tokio::process::Child,
    output_dir: PathBuf,
    stderr: Arc<Mutex<StderrTail>>,
}

impl FfmpegRunner {
    /// Spawn FFmpeg producing HLS into `config.output_dir`.
    pub fn start(config: &HlsConfig) -> Result<Self, AppError> {
        let args = build_args(config);
        let mut child = tokio::process::Command::new(&config.ffmpeg_path)
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                AppError::internal(format!("ffmpeg spawn ({}) : {e}", config.ffmpeg_path))
            })?;

        let stderr = Arc::new(Mutex::new(StderrTail::new(8 * 1024)));
        if let Some(mut reader) = child.stderr.take() {
            let tail = stderr.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                loop {
                    match reader.read(&mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if let Ok(mut t) = tail.lock() {
                                t.push(&buf[..n]);
                            }
                        }
                    }
                }
            });
        }

        Ok(Self {
            child,
            output_dir: config.output_dir.clone(),
            stderr,
        })
    }

    /// Wait until at least one `.ts` segment exists, or the process exits, or
    /// `timeout` elapses. On failure the bounded stderr tail is included.
    pub async fn wait_ready(&mut self, timeout: Duration) -> Result<(), AppError> {
        let deadline = Instant::now() + timeout;
        loop {
            if has_segment(&self.output_dir) {
                return Ok(());
            }
            if let Some(status) = self
                .child
                .try_wait()
                .map_err(|e| AppError::internal(format!("ffmpeg wait: {e}")))?
            {
                let tail = stderr_tail(&self.stderr);
                return Err(AppError::internal(format!(
                    "ffmpeg exited before ready: {status}: {tail}"
                )));
            }
            if Instant::now() >= deadline {
                let tail = stderr_tail(&self.stderr);
                return Err(AppError::internal(format!(
                    "timed out waiting for HLS segment: {tail}"
                )));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Stop FFmpeg gracefully (SIGTERM, then SIGKILL after a grace period) and
    /// reap the child. Idempotent.
    pub async fn stop(&mut self) {
        if let Some(pid) = self.child.id() {
            // SIGTERM lets FFmpeg flush the playlist/segments (STREAMING.md §12.4).
            unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        }
        if tokio::time::timeout(Duration::from_secs(5), self.child.wait())
            .await
            .is_ok()
        {
            return;
        }
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}

fn stderr_tail(stderr: &Arc<Mutex<StderrTail>>) -> String {
    stderr
        .lock()
        .map(|t| t.as_string())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn has_segment(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .any(|e| e.path().extension().is_some_and(|x| x == "ts"))
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_include_input_output_and_copy() {
        let cfg = HlsConfig {
            ffmpeg_path: "ffmpeg".into(),
            input_url: "http://127.0.0.1:1234/stream/s1".into(),
            output_dir: "/tmp/sess".into(),
            start_time_secs: None,
        };
        let args = build_args(&cfg);
        let joined = args.join(" ");
        assert!(joined.contains("-i http://127.0.0.1:1234/stream/s1"));
        assert!(joined.contains("-c copy"));
        assert!(joined.contains("-f hls"));
        assert!(joined.contains("/tmp/sess/out.m3u8"));
        assert!(!joined.contains("-ss"));
    }

    #[test]
    fn seek_adds_ss_before_input() {
        let cfg = HlsConfig {
            ffmpeg_path: "ffmpeg".into(),
            input_url: "http://x/".into(),
            output_dir: "/tmp/sess".into(),
            start_time_secs: Some(5.0),
        };
        let args = build_args(&cfg);
        let ss = args.iter().position(|a| a == "-ss").unwrap();
        let input = args.iter().position(|a| a == "-i").unwrap();
        assert!(ss < input, "-ss must precede -i for input seek");
        assert_eq!(args[ss + 1], "5");
    }

    #[test]
    fn stderr_tail_is_bounded() {
        let mut tail = StderrTail::new(16);
        tail.push(b"0123456789");
        tail.push(b"ABCDEFGHIJ");
        assert_eq!(tail.as_string(), "456789ABCDEFGHIJ"); // first 6 bytes evicted
    }
}
