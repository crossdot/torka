//! Streaming subsystem: torrent resolution, a seekable loopback source server,
//! FFprobe inspection, playback planning, and the FFmpeg HLS runner.

use tokio::io::{AsyncRead, AsyncSeek};

/// A readable, seekable, `Send + Unpin` byte stream. Satisfied by librqbit's
/// `FileStream` (whose concrete type is not nameable, so it is boxed behind
/// this trait object).
pub trait SeekableStream: AsyncRead + AsyncSeek + Send + Unpin {}
impl<T: AsyncRead + AsyncSeek + Send + Unpin> SeekableStream for T {}

pub mod ffmpeg;
pub mod manager;
pub mod memory_storage;
pub mod plan;
pub mod probe;
pub mod session;
pub mod source;
pub mod torrent;

pub use ffmpeg::{Encode, FfmpegRunner, HlsConfig};
pub use manager::StreamingManager;
pub use plan::PlaybackMode;
pub use probe::StreamInfo;
pub use session::SessionState;
pub use source::SourceServer;
pub use torrent::{TorrentEngine, TorrentFile, TorrentSource};
