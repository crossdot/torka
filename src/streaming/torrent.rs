//! Torrent resolver: a shared librqbit session that resolves magnet/`.torrent`
//! sources, enumerates files, and yields seekable file streams.

use std::{path::PathBuf, sync::Arc};

use librqbit::{
    AddTorrent, AddTorrentOptions, ManagedTorrent, Session, storage::StorageFactoryExt,
};

use crate::error::AppError;

use super::{SeekableStream, memory_storage::MemoryStorageFactory};

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
    stream_buffer_bytes: usize,
}

impl TorrentEngine {
    pub async fn new(
        download_dir: impl Into<PathBuf>,
        stream_buffer_bytes: usize,
    ) -> Result<Self, AppError> {
        let session = Session::new(download_dir.into())
            .await
            .map_err(|e| AppError::internal(format!("torrent session: {e}")))?;
        Ok(Self {
            session,
            stream_buffer_bytes,
        })
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

        // Streaming-only: `only_files: []` empties librqbit's background download
        // queue, so only the FileStream's priority window (the playback buffer) is
        // fetched — the whole torrent is never downloaded.
        //
        // In production (`output_folder: None`) data lives only in the bounded
        // in-memory storage (no file cache). The PoC keeps filesystem storage so
        // it can seed a local test file via `output_folder`.
        let output_folder = output_folder.map(|p| p.to_string_lossy().into_owned());
        let storage_factory = output_folder
            .is_none()
            .then(|| MemoryStorageFactory::new(self.stream_buffer_bytes).boxed());
        let opts = Some(AddTorrentOptions {
            output_folder,
            overwrite: true,
            only_files: Some(vec![]),
            storage_factory,
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

    /// Remove a torrent from the shared session. Called on session end so a
    /// later reopen of the same source starts fresh (the in-memory storage is
    /// otherwise reused with evicted pieces).
    pub async fn delete(&self, handle: &Arc<ManagedTorrent>) -> Result<(), AppError> {
        self.session
            .delete(librqbit::api::TorrentIdOrHash::Id(handle.id()), false)
            .await
            .map_err(|e| AppError::internal(format!("delete torrent: {e}")))
    }
}
