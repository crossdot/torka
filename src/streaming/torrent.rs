//! Torrent resolver: a shared librqbit session that resolves magnet/`.torrent`
//! sources, enumerates files, and yields seekable file streams.

use std::{path::PathBuf, sync::Arc};

use librqbit::{AddTorrent, AddTorrentOptions, ManagedTorrent, Session};

use crate::error::AppError;

use super::SeekableStream;

/// A torrent source to resolve.
pub enum TorrentSource {
    Magnet(String),
    Bytes(Vec<u8>),
}

impl TorrentSource {
    /// Build a magnet source, validating that it carries a valid BTIH.
    pub fn from_magnet(uri: String) -> Result<Self, AppError> {
        if crate::providers::magnet::magnet_info_hash(&uri).is_none() {
            return Err(AppError::validation("invalid magnet URI (no valid btih)"));
        }
        Ok(Self::Magnet(uri))
    }
}

/// A single file within a torrent.
#[derive(Debug, Clone)]
pub struct TorrentFile {
    pub index: usize,
    pub name: String,
    pub size_bytes: u64,
}

/// Wraps a librqbit [`Session`] for metadata resolution and streaming.
pub struct TorrentEngine {
    session: Arc<Session>,
}

impl TorrentEngine {
    pub async fn new(download_dir: impl Into<PathBuf>) -> Result<Self, AppError> {
        let session = Session::new(download_dir.into())
            .await
            .map_err(|e| AppError::internal(format!("torrent session: {e}")))?;
        Ok(Self { session })
    }

    /// Add a torrent and wait for its metadata to resolve. `output_folder`
    /// overrides where files are stored (used by the PoC to detect an
    /// already-present file).
    pub async fn add(
        &self,
        source: TorrentSource,
        output_folder: Option<PathBuf>,
    ) -> Result<Arc<ManagedTorrent>, AppError> {
        let add = match source {
            TorrentSource::Magnet(uri) => AddTorrent::from_url(uri),
            TorrentSource::Bytes(bytes) => AddTorrent::from_bytes(bytes),
        };

        let opts = output_folder.map(|p| AddTorrentOptions {
            output_folder: Some(p.to_string_lossy().into_owned()),
            // Accept a pre-existing file (hash it as already downloaded); used by
            // the PoC to seed a local test torrent without a live peer.
            overwrite: true,
            ..Default::default()
        });

        let resp = self
            .session
            .add_torrent(add, opts)
            .await
            .map_err(|e| AppError::internal(format!("add torrent: {e}")))?;

        let handle = resp
            .into_handle()
            .ok_or_else(|| AppError::internal("torrent already managed or list-only"))?;

        handle
            .wait_until_initialized()
            .await
            .map_err(|e| AppError::internal(format!("resolve metadata: {e}")))?;

        Ok(handle)
    }

    /// Enumerate files (name + size), largest first (video selection heuristic
    /// is refined later).
    pub async fn files(&self, handle: &Arc<ManagedTorrent>) -> Result<Vec<TorrentFile>, AppError> {
        handle
            .with_metadata(|m| {
                let mut files: Vec<TorrentFile> = m
                    .file_infos
                    .iter()
                    .enumerate()
                    .map(|(i, f)| TorrentFile {
                        index: i,
                        name: f.relative_filename.to_string_lossy().into_owned(),
                        size_bytes: f.len,
                    })
                    .collect();
                files.sort_by_key(|a| std::cmp::Reverse(a.size_bytes));
                files
            })
            .map_err(|e| AppError::internal(format!("list files: {e}")))
    }

    /// Open a seekable stream for `file_id`.
    pub async fn stream(
        &self,
        handle: &Arc<ManagedTorrent>,
        file_id: usize,
    ) -> Result<Box<dyn SeekableStream>, AppError> {
        let stream = handle
            .clone()
            .stream(file_id)
            .await
            .map_err(|e| AppError::internal(format!("open stream: {e}")))?;
        Ok(Box::new(stream))
    }
}
