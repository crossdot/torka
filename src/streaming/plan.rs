//! Playback planning: choose remux vs transcode from probed codecs, the client
//! preference, and the configured hardware-acceleration backend.

use crate::{config::HwAccelBackend, error::AppError};

use super::{ffmpeg::Encode, probe::StreamInfo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackMode {
    Remux,
    Transcode(Encode),
}

/// Select a playback mode.
///
/// `preferred` is the client's `preferred_mode` (`"auto"`, `"remux"`, or
/// `"transcode"`). `"auto"`/`None` remuxes when the codecs are HLS-compatible
/// and transcodes otherwise.
pub fn select_mode(
    preferred: Option<&str>,
    info: &StreamInfo,
    backend: HwAccelBackend,
    device: Option<&str>,
) -> Result<PlaybackMode, AppError> {
    match preferred {
        Some("remux") => Ok(PlaybackMode::Remux),
        Some("transcode") => Ok(PlaybackMode::Transcode(encoder(backend, device))),
        Some(other) => Err(AppError::validation(format!(
            "unknown preferred_mode: {other}"
        ))),
        None => Ok(if is_hls_compatible(info) {
            PlaybackMode::Remux
        } else {
            PlaybackMode::Transcode(encoder(backend, device))
        }),
    }
}

fn encoder(backend: HwAccelBackend, device: Option<&str>) -> Encode {
    match backend {
        HwAccelBackend::Vaapi => Encode::Vaapi(device.unwrap_or("/dev/dri/renderD128").to_string()),
        HwAccelBackend::Nvenc => Encode::Nvenc,
        // `auto` currently picks software; runtime GPU probing is a follow-up.
        HwAccelBackend::Auto | HwAccelBackend::Software => Encode::Software,
    }
}

/// HLS (MPEG-TS) carries H.264 + AAC/MP3 without re-encoding; other codecs need
/// transcoding.
fn is_hls_compatible(info: &StreamInfo) -> bool {
    let video = info.video_codec.as_deref().unwrap_or("");
    let audio = info.audio_codec.as_deref().unwrap_or("");
    let video_ok = video.is_empty() || matches!(video, "h264" | "avc1");
    let audio_ok = audio.is_empty() || matches!(audio, "aac" | "mp3");
    video_ok && audio_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(video: &str, audio: &str) -> StreamInfo {
        StreamInfo {
            video_codec: Some(video.to_string()),
            audio_codec: Some(audio.to_string()),
        }
    }

    #[test]
    fn auto_remuxes_h264_aac() {
        let mode = select_mode(None, &info("h264", "aac"), HwAccelBackend::Auto, None).unwrap();
        assert_eq!(mode, PlaybackMode::Remux);
    }

    #[test]
    fn auto_transcodes_incompatible_codecs() {
        let mode = select_mode(None, &info("av1", "opus"), HwAccelBackend::Auto, None).unwrap();
        assert_eq!(mode, PlaybackMode::Transcode(Encode::Software));
    }

    #[test]
    fn forced_remux_ignores_codecs() {
        let mode = select_mode(
            Some("remux"),
            &info("av1", "opus"),
            HwAccelBackend::Auto,
            None,
        )
        .unwrap();
        assert_eq!(mode, PlaybackMode::Remux);
    }

    #[test]
    fn forced_transcode_uses_configured_backend() {
        let mode = select_mode(
            Some("transcode"),
            &info("h264", "aac"),
            HwAccelBackend::Nvenc,
            None,
        )
        .unwrap();
        assert_eq!(mode, PlaybackMode::Transcode(Encode::Nvenc));
    }
}
