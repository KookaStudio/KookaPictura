//! The scratch disk (#1217, `docs/10-workflow-io/scratch-disks-and-memory.md`):
//! payloads evicted from RAM in checksummed extents of an unnamed file on each
//! of up to four volumes, with typed errors, failure injection, and the
//! Scratch Sizes and Efficiency readouts.
//!
//! I/O is `pread`/`pwrite` (`FileExt`), not `mmap`: a failed read is an `Err`,
//! not a `SIGBUS`, and written pages are reclaimable page cache (design R8).

mod efficiency;
mod extents;
mod fault;
mod volume;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub use efficiency::{efficiency, io_timer, operation, Efficiency, IoTimer, Operation};
pub use fault::{clear_faults, inject, Fault};
pub use volume::{default_volume, VolumeInfo};

use volume::Volume;

/// The most volumes the store uses (the spec's ordered list of four).
pub const MAX_VOLUMES: usize = 4;

/// Why the scratch disk could not take or give back a payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScratchError {
    /// Every volume is full (or removed).
    Full,
    /// The extent could not be read back intact.
    Unreadable,
}

impl std::fmt::Display for ScratchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => f.write_str("the scratch disks are full"),
            Self::Unreadable => f.write_str("image data could not be read back from scratch"),
        }
    }
}

impl std::error::Error for ScratchError {}

/// Where one payload lives on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    volume: u16,
    offset: u64,
    len: u32,
    checksum: u32,
}

impl Extent {
    /// The payload's length in bytes.
    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The bytes the extent occupies in its file.
    pub fn stored(&self) -> u64 {
        extents::aligned(u64::from(self.len))
    }
}

/// Scratch Sizes: the bytes held in extents and the bytes still available.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sizes {
    pub used: u64,
    pub available: u64,
}

/// A set of scratch volumes. Extents name their volume by its index in an
/// append-only list, so changing the configured volumes never strands one.
pub struct Store {
    volumes: Vec<Volume>,
    /// The configured volumes, in order of preference.
    order: Vec<usize>,
}

impl Store {
    /// A store on `dirs` (at most [`MAX_VOLUMES`]); files open on first use.
    pub fn new(dirs: &[PathBuf]) -> Self {
        let mut store = Self {
            volumes: Vec::new(),
            order: Vec::new(),
        };
        store.set_volumes(dirs);
        store
    }

    /// Use `dirs` for new extents from now on. Extents on volumes left out
    /// stay readable until freed.
    pub fn set_volumes(&mut self, dirs: &[PathBuf]) {
        self.order.clear();
        for dir in dirs.iter().take(MAX_VOLUMES) {
            let at = match self.volumes.iter().position(|v| v.dir() == dir.as_path()) {
                Some(at) => at,
                None => {
                    self.volumes.push(Volume::new(dir.clone()));
                    self.volumes.len() - 1
                }
            };
            if !self.order.contains(&at) {
                self.order.push(at);
            }
        }
    }

    /// Write `payload` to the first configured volume with room.
    pub fn write(&mut self, payload: &[u8]) -> Result<Extent, ScratchError> {
        let checksum = crc32fast::hash(payload);
        for (rank, &at) in self.order.iter().enumerate() {
            let Some(offset) = self.volumes[at].allocate(rank, payload.len() as u64) else {
                continue;
            };
            let extent = Extent {
                volume: at as u16,
                offset,
                len: payload.len() as u32,
                checksum,
            };
            if self.volumes[at].write(rank, offset, payload).is_ok() {
                return Ok(extent);
            }
            self.volumes[at].release(offset, extent.stored());
            self.volumes[at].mark_full();
        }
        Err(ScratchError::Full)
    }

    /// Read `extent`'s payload into `out` (replacing its contents) and check
    /// it against the extent's checksum.
    pub fn read(&self, extent: &Extent, out: &mut Vec<u8>) -> Result<(), ScratchError> {
        let volume = self
            .volumes
            .get(usize::from(extent.volume))
            .ok_or(ScratchError::Unreadable)?;
        let rank = self
            .order
            .iter()
            .position(|&at| at == usize::from(extent.volume));
        out.clear();
        out.resize(extent.len(), 0);
        volume
            .read(rank, extent.offset, out)
            .map_err(|_| ScratchError::Unreadable)?;
        if crc32fast::hash(out) != extent.checksum {
            return Err(ScratchError::Unreadable);
        }
        Ok(())
    }

    /// Give `extent`'s space back to its volume.
    pub fn free(&mut self, extent: Extent) {
        if let Some(volume) = self.volumes.get_mut(usize::from(extent.volume)) {
            volume.release(extent.offset, extent.stored());
        }
    }

    pub fn sizes(&self) -> Sizes {
        let used = self.volumes.iter().map(Volume::used).sum();
        let available = self
            .order
            .iter()
            .enumerate()
            .filter(|&(rank, &at)| !self.volumes[at].unusable(rank))
            .map(|(_, &at)| self.volumes[at].available())
            .sum();
        Sizes { used, available }
    }

    /// The configured volumes, in order.
    pub fn volumes(&self) -> Vec<VolumeInfo> {
        self.order
            .iter()
            .enumerate()
            .map(|(rank, &at)| self.volumes[at].info(rank))
            .collect()
    }

    /// The configured directories, in order.
    pub fn dirs(&self) -> Vec<PathBuf> {
        self.order
            .iter()
            .map(|&at| self.volumes[at].dir().to_path_buf())
            .collect()
    }

    /// Bytes held in the files' extents, per configured volume.
    pub fn file_len(&self, rank: usize) -> u64 {
        self.order
            .get(rank)
            .map_or(0, |&at| self.volumes[at].file_len())
    }
}

/// The process's store, on the default volume until configured.
pub fn store() -> &'static Mutex<Store> {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(Store::new(&[default_volume()])))
}

/// Volumes set while the store was busy; whoever takes the store next applies
/// them, so the GUI thread never waits behind a spill to change volumes (#636).
static PENDING: Mutex<Option<Vec<PathBuf>>> = Mutex::new(None);

/// The last readouts, served while the store is busy.
static LAST_READOUT: Mutex<Option<(Vec<VolumeInfo>, Sizes)>> = Mutex::new(None);

fn apply_pending(store: &mut Store) {
    let pending = PENDING.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(dirs) = pending {
        store.set_volumes(&dirs);
    }
}

fn locked() -> std::sync::MutexGuard<'static, Store> {
    let mut store = store().lock().unwrap_or_else(|e| e.into_inner());
    apply_pending(&mut store);
    store
}

/// The store when no other thread holds it.
fn try_locked() -> Option<std::sync::MutexGuard<'static, Store>> {
    let mut store = match store().try_lock() {
        Ok(store) => store,
        Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner(),
        Err(std::sync::TryLockError::WouldBlock) => return None,
    };
    apply_pending(&mut store);
    Some(store)
}

/// Configure the process's volumes (an empty list keeps the default). Never
/// waits for the store: while another thread holds it, the change applies
/// when that thread is done.
pub fn set_volumes(dirs: &[PathBuf]) {
    let dirs = if dirs.is_empty() {
        vec![default_volume()]
    } else {
        dirs.to_vec()
    };
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(dirs);
    drop(try_locked());
}

/// The volumes and Scratch Sizes, read without waiting while the store is
/// busy: the last successful reading stands in, even when a volume change is
/// pending (it shows at the first reading after the store frees). Only the
/// very first reading waits.
fn readout() -> (Vec<VolumeInfo>, Sizes) {
    let mut last = LAST_READOUT.lock().unwrap_or_else(|e| e.into_inner());
    let store = match try_locked() {
        Some(store) => store,
        None => match last.as_ref() {
            Some(reading) => return reading.clone(),
            None => locked(),
        },
    };
    let reading = (store.volumes(), store.sizes());
    *last = Some(reading.clone());
    reading
}

/// The process's configured volumes.
pub fn volumes() -> Vec<VolumeInfo> {
    readout().0
}

/// The process's Scratch Sizes.
pub fn sizes() -> Sizes {
    readout().1
}

/// Write `payload` to the process's store, timed as scratch I/O.
pub fn write(payload: &[u8]) -> Result<Extent, ScratchError> {
    let _io = io_timer();
    locked().write(payload)
}

/// Read `extent` from the process's store into `out`, timed as scratch I/O.
///
/// The lock is held across the read: concurrent restores of different tiles
/// serialise on it, which the disk would do anyway.
pub fn read(extent: &Extent, out: &mut Vec<u8>) -> Result<(), ScratchError> {
    let _io = io_timer();
    locked().read(extent, out)
}

/// Free `extent` in the process's store.
pub fn free(extent: Extent) {
    locked().free(extent);
}

/// Whether `dir` is on a RAM-backed filesystem (`tmpfs`).
pub fn is_tmpfs(dir: &Path) -> bool {
    volume::is_tmpfs(dir)
}

#[cfg(test)]
pub(crate) mod tests;
