//! Streaming subsystem: torrent resolution, a seekable loopback source server,
//! and the FFmpeg HLS runner. This is the engine proven by the Phase 0.5 spike
//! (ADR-002), structured as reusable modules for the Phase 6 playback API.

use tokio::io::{AsyncRead, AsyncSeek};

/// A readable, seekable, `Send + Unpin` byte stream. Satisfied by librqbit's
/// `FileStream` (whose concrete type is not nameable, so it is boxed behind
/// this trait object).
pub trait SeekableStream: AsyncRead + AsyncSeek + Send + Unpin {}
impl<T: AsyncRead + AsyncSeek + Send + Unpin> SeekableStream for T {}

pub mod ffmpeg;
pub mod source;
pub mod torrent;

pub use ffmpeg::{FfmpegRunner, HlsConfig};
pub use source::SourceServer;
pub use torrent::{TorrentEngine, TorrentFile, TorrentSource};
