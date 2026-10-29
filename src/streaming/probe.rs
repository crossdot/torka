//! FFprobe inspection of the selected media source: determines container/codec
//! compatibility to decide between remux and transcode.

use std::time::Duration;

use serde::Deserialize;

use crate::error::AppError;

/// Video/audio codec names + total duration of a media source.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StreamInfo {
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    /// Total media duration in seconds (from the container `format`), when
    /// ffprobe can determine it.
    pub duration_secs: Option<f64>,
    /// Whether the source has a subtitle stream.
    pub has_subtitles: bool,
}

#[derive(Deserialize)]
struct FfprobeOutput {
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
}

#[derive(Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

/// Run `ffprobe` on `input_url` and return the first video/audio codecs.
pub async fn probe(
    ffprobe_path: &str,
    input_url: &str,
    timeout: Duration,
) -> Result<StreamInfo, AppError> {
    let mut cmd = tokio::process::Command::new(ffprobe_path);
    cmd.args([
        "-v",
        "error",
        "-show_entries",
        "stream=codec_type,codec_name:format=duration",
        "-of",
        "json",
        input_url,
    ])
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::null());

    let output = tokio::time::timeout(timeout, cmd.output())
        .await
        .map_err(|_| AppError::internal("ffprobe timed out"))?
        .map_err(|e| AppError::internal(format!("ffprobe spawn: {e}")))?;

    if !output.status.success() {
        return Err(AppError::internal(format!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&output.stdout)
        )));
    }

    parse_streams(&output.stdout)
}

/// Parse `ffprobe -of json` output into [`StreamInfo`]. Pure for testability.
fn parse_streams(json: &[u8]) -> Result<StreamInfo, AppError> {
    let parsed: FfprobeOutput = serde_json::from_slice(json)
        .map_err(|e| AppError::internal(format!("ffprobe parse: {e}")))?;

    let find = |ty: &str| {
        parsed
            .streams
            .iter()
            .find(|s| s.codec_type.as_deref() == Some(ty))
            .and_then(|s| s.codec_name.clone())
    };

    let duration_secs = parsed
        .format
        .as_ref()
        .and_then(|f| f.duration.as_deref())
        .and_then(|d| d.parse::<f64>().ok());

    let has_subtitles = parsed
        .streams
        .iter()
        .any(|s| s.codec_type.as_deref() == Some("subtitle"));

    Ok(StreamInfo {
        video_codec: find("video"),
        audio_codec: find("audio"),
        duration_secs,
        has_subtitles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_video_and_audio_codecs() {
        let json = br#"{"streams":[
            {"codec_type":"video","codec_name":"h264"},
            {"codec_type":"audio","codec_name":"aac"}
        ]}"#;
        let info = parse_streams(json).unwrap();
        assert_eq!(info.video_codec.as_deref(), Some("h264"));
        assert_eq!(info.audio_codec.as_deref(), Some("aac"));
    }

    #[test]
    fn handles_unsupported_codecs() {
        let json = br#"{"streams":[{"codec_type":"video","codec_name":"av1"}]}"#;
        let info = parse_streams(json).unwrap();
        assert_eq!(info.video_codec.as_deref(), Some("av1"));
        assert_eq!(info.audio_codec, None);
    }
}
