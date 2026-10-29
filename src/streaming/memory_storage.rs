//! Bounded in-memory torrent storage (no disk): keeps only a sliding window of
//! pieces around the read position, evicting the least-recently-used ones.
//!
//! Combined with `only_files: []` (which empties librqbit's background download
//! queue so only the FileStream's priority window is fetched), this replaces the
//! default filesystem storage and its unbounded sparse-file cache with a fixed
//! memory budget.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Mutex,
};

use anyhow::Context;
use librqbit::{
    FileInfos, ManagedTorrentShared, TorrentMetadata,
    storage::{BoxStorageFactory, StorageFactory, StorageFactoryExt, TorrentStorage},
};

struct Piece {
    data: Box<[u8]>,
    /// Monotonic access counter for LRU eviction.
    last_access: u64,
}

struct Inner {
    pieces: HashMap<u32, Piece>,
    total_bytes: usize,
    clock: u64,
}

/// Factory for [`MemoryStorage`]. `max_bytes` is the per-torrent memory budget.
#[derive(Clone)]
pub struct MemoryStorageFactory {
    max_bytes: usize,
}

impl MemoryStorageFactory {
    pub fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }
}

impl StorageFactory for MemoryStorageFactory {
    type Storage = MemoryStorage;

    fn create(
        &self,
        _shared: &ManagedTorrentShared,
        metadata: &TorrentMetadata,
    ) -> anyhow::Result<MemoryStorage> {
        let piece_len = metadata.lengths().default_piece_length() as usize;
        // Keep at least a few pieces so a single large piece can't thrash the
        // buffer (evicting the piece we just wrote).
        let max_bytes = self.max_bytes.max(piece_len.saturating_mul(8));
        // Pin the first and last piece of every file: FFmpeg re-reads a file's
        // header (start) and its seek index (end) on every restart/seek; if
        // those were LRU-evicted the reopen fails with "End of file".
        let pinned: HashSet<u32> = metadata
            .file_infos
            .iter()
            .filter(|f| f.len > 0)
            .flat_map(|f| {
                let plen = piece_len as u64;
                let first = (f.offset_in_torrent / plen) as u32;
                let last = ((f.offset_in_torrent + f.len - 1) / plen) as u32;
                [first, last]
            })
            .collect();
        Ok(MemoryStorage {
            file_infos: metadata.file_infos.clone(),
            piece_len,
            max_bytes,
            pinned,
            inner: Mutex::new(Inner {
                pieces: HashMap::new(),
                total_bytes: 0,
                clock: 0,
            }),
        })
    }

    fn clone_box(&self) -> BoxStorageFactory {
        self.clone().boxed()
    }
}

pub struct MemoryStorage {
    file_infos: FileInfos,
    piece_len: usize,
    max_bytes: usize,
    pinned: HashSet<u32>,
    inner: Mutex<Inner>,
}

impl MemoryStorage {
    fn piece_index(&self, file_id: usize, offset: u64) -> u32 {
        let fi = &self.file_infos[file_id];
        ((fi.offset_in_torrent + offset) / self.piece_len as u64) as u32
    }

    fn piece_offset(&self, file_id: usize, offset: u64) -> usize {
        let fi = &self.file_infos[file_id];
        ((fi.offset_in_torrent + offset) % self.piece_len as u64) as usize
    }
}

impl TorrentStorage for MemoryStorage {
    fn init(
        &mut self,
        _shared: &ManagedTorrentShared,
        _metadata: &TorrentMetadata,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn pread_exact(&self, file_id: usize, offset: u64, buf: &mut [u8]) -> anyhow::Result<()> {
        let idx = self.piece_index(file_id, offset);
        let off = self.piece_offset(file_id, offset);
        let mut g = self.inner.lock().unwrap();
        let Inner { pieces, clock, .. } = &mut *g;
        let piece = pieces
            .get_mut(&idx)
            .context("stream buffer evicted this piece (seek too far back)")?;
        if off + buf.len() > piece.data.len() {
            anyhow::bail!("piece read out of bounds");
        }
        buf.copy_from_slice(&piece.data[off..off + buf.len()]);
        piece.last_access = *clock;
        *clock += 1;
        Ok(())
    }

    fn pwrite_all(&self, file_id: usize, offset: u64, buf: &[u8]) -> anyhow::Result<()> {
        let idx = self.piece_index(file_id, offset);
        let off = self.piece_offset(file_id, offset);
        let mut g = self.inner.lock().unwrap();
        let Inner {
            pieces,
            total_bytes,
            clock,
        } = &mut *g;

        let piece = pieces.entry(idx).or_insert_with(|| {
            *total_bytes += self.piece_len;
            Piece {
                data: vec![0u8; self.piece_len].into_boxed_slice(),
                last_access: 0,
            }
        });
        if off + buf.len() > piece.data.len() {
            anyhow::bail!("piece write out of bounds");
        }
        piece.data[off..off + buf.len()].copy_from_slice(buf);
        piece.last_access = *clock;
        *clock += 1;

        // Evict least-recently-used pieces while over budget, never the pinned
        // header/index pieces.
        while *total_bytes > self.max_bytes {
            let victim = pieces
                .iter()
                .filter(|(k, _)| !self.pinned.contains(k))
                .min_by_key(|(_, p)| p.last_access)
                .map(|(k, _)| *k);
            let Some(v) = victim else { break };
            if let Some(removed) = pieces.remove(&v) {
                *total_bytes -= removed.data.len();
            }
        }
        Ok(())
    }

    fn remove_file(&self, _file_id: usize, _filename: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    fn remove_directory_if_empty(&self, _path: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    fn ensure_file_length(&self, _file_id: usize, _length: u64) -> anyhow::Result<()> {
        Ok(())
    }

    fn take(&self) -> anyhow::Result<Box<dyn TorrentStorage>> {
        let inner = {
            let mut g = self.inner.lock().unwrap();
            std::mem::replace(
                &mut *g,
                Inner {
                    pieces: HashMap::new(),
                    total_bytes: 0,
                    clock: 0,
                },
            )
        };
        Ok(Box::new(MemoryStorage {
            file_infos: self.file_infos.clone(),
            piece_len: self.piece_len,
            max_bytes: self.max_bytes,
            pinned: self.pinned.clone(),
            inner: Mutex::new(inner),
        }))
    }
}
