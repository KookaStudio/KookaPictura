//! One scratch volume: an unnamed file in a directory, grown with
//! `fallocate`, its free space, and the platform calls behind it.

use std::fs::File;
use std::path::{Path, PathBuf};

use super::extents::{aligned, FreeMap};
use super::fault;

/// How much a file grows when no free range fits.
const GROW: u64 = 32 * 1024 * 1024;

/// One configured volume, for the readouts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeInfo {
    pub path: PathBuf,
    /// The scratch file's size.
    pub file_len: u64,
    /// Bytes held in extents.
    pub used: u64,
    /// Free space on the filesystem plus free extents in the file.
    pub available: u64,
    /// Full, removed, or unusable: new extents go elsewhere.
    pub full: bool,
    /// On a RAM-backed filesystem: scratch here consumes memory (warning).
    pub tmpfs: bool,
}

/// `$XDG_CACHE_HOME/kooka-pictura/scratch`, or `~/.cache/…` without it.
pub fn default_volume() -> PathBuf {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir);
    cache.join("kooka-pictura").join("scratch")
}

pub(super) struct Volume {
    dir: PathBuf,
    file: Option<File>,
    /// The file could not be opened: the volume takes nothing.
    broken: bool,
    file_len: u64,
    free: FreeMap,
    used: u64,
    full: bool,
}

impl Volume {
    pub(super) fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            file: None,
            broken: false,
            file_len: 0,
            free: FreeMap::default(),
            used: 0,
            full: false,
        }
    }

    pub(super) fn dir(&self) -> &Path {
        &self.dir
    }

    pub(super) fn used(&self) -> u64 {
        self.used
    }

    pub(super) fn file_len(&self) -> u64 {
        self.file_len
    }

    /// Whether the volume at `rank` takes no new extents.
    pub(super) fn unusable(&self, rank: usize) -> bool {
        self.full || self.broken || fault::full(Some(rank)) || fault::removed(Some(rank))
    }

    pub(super) fn mark_full(&mut self) {
        self.full = true;
    }

    pub(super) fn available(&self) -> u64 {
        filesystem_available(&self.dir).unwrap_or(0) + self.free.bytes()
    }

    pub(super) fn info(&self, rank: usize) -> VolumeInfo {
        VolumeInfo {
            path: self.dir.clone(),
            file_len: self.file_len,
            used: self.used,
            available: if self.unusable(rank) {
                0
            } else {
                self.available()
            },
            full: self.unusable(rank),
            tmpfs: is_tmpfs(&self.dir) || self.dir.starts_with("/tmp"),
        }
    }

    /// Room for `len` payload bytes, growing the file when no free range
    /// fits; `None` when the volume is full.
    pub(super) fn allocate(&mut self, rank: usize, len: u64) -> Option<u64> {
        if self.unusable(rank) {
            return None;
        }
        let len = aligned(len);
        if let Some(offset) = self.free.take(len) {
            self.used += len;
            return Some(offset);
        }
        if self.file.is_none() {
            match open_unnamed(&self.dir) {
                Ok(file) => self.file = Some(file),
                Err(_) => {
                    self.broken = true;
                    return None;
                }
            }
        }
        let grow = len.max(GROW);
        let file = self.file.as_ref()?;
        if reserve(file, self.file_len, grow).is_err() {
            self.full = true;
            return None;
        }
        self.free.give(self.file_len, grow);
        self.file_len += grow;
        let offset = self.free.take(len)?;
        self.used += len;
        Some(offset)
    }

    /// Give back `len` aligned bytes at `offset`; a full volume takes extents
    /// again once one is freed.
    pub(super) fn release(&mut self, offset: u64, len: u64) {
        self.free.give(offset, len);
        self.used = self.used.saturating_sub(len);
        self.full = false;
    }

    pub(super) fn write(&self, rank: usize, offset: u64, payload: &[u8]) -> std::io::Result<()> {
        let file = self.file.as_ref().ok_or_else(gone)?;
        if fault::removed(Some(rank)) {
            return Err(gone());
        }
        if fault::corrupt() && !payload.is_empty() {
            let mut bad = payload.to_vec();
            bad[0] ^= 0xff;
            return write_at(file, &bad, offset);
        }
        write_at(file, payload, offset)
    }

    /// `rank` is the volume's place among the configured ones, `None` when it
    /// is no longer configured.
    pub(super) fn read(
        &self,
        rank: Option<usize>,
        offset: u64,
        out: &mut [u8],
    ) -> std::io::Result<()> {
        let file = self.file.as_ref().ok_or_else(gone)?;
        if fault::read_error() || fault::removed(rank) {
            return Err(std::io::Error::other("injected scratch read error"));
        }
        read_at(file, out, offset)
    }
}

fn gone() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::NotFound, "scratch volume is gone")
}

/// Open an unnamed file in `dir`: `O_TMPFILE` where the filesystem supports
/// it, else a named file unlinked at once. Either way a crash leaves nothing
/// (Parity criterion 10). Named files a dead process left are removed first.
fn open_unnamed(dir: &Path) -> std::io::Result<File> {
    std::fs::create_dir_all(dir)?;
    sweep_leftovers(dir);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let unnamed = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_TMPFILE)
            .open(dir);
        if let Ok(file) = unnamed {
            return Ok(file);
        }
    }
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = dir.join(format!("pictura-scratch-{}-{n}", std::process::id()));
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)?;
    std::fs::remove_file(&path)?;
    Ok(file)
}

/// Remove `pictura-scratch-<pid>-<n>` files whose process is gone.
fn sweep_leftovers(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(rest) = name
            .to_str()
            .and_then(|n| n.strip_prefix("pictura-scratch-"))
        else {
            continue;
        };
        let Some(pid) = rest.split('-').next().and_then(|p| p.parse::<u32>().ok()) else {
            continue;
        };
        if pid != std::process::id() && !Path::new(&format!("/proc/{pid}")).exists() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(unix)]
fn write_at(file: &File, buf: &[u8], offset: u64) -> std::io::Result<()> {
    std::os::unix::fs::FileExt::write_all_at(file, buf, offset)
}

#[cfg(unix)]
fn read_at(file: &File, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
    std::os::unix::fs::FileExt::read_exact_at(file, buf, offset)
}

#[cfg(windows)]
fn write_at(file: &File, mut buf: &[u8], mut offset: u64) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        let n = file.seek_write(buf, offset)?;
        buf = &buf[n..];
        offset += n as u64;
    }
    Ok(())
}

#[cfg(windows)]
fn read_at(file: &File, mut buf: &mut [u8], mut offset: u64) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        let n = file.seek_read(buf, offset)?;
        if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        buf = &mut buf[n..];
        offset += n as u64;
    }
    Ok(())
}

/// Reserve `len` bytes at `offset` on disk, so a full filesystem shows up
/// here rather than as a failed write later.
#[cfg(target_os = "linux")]
fn reserve(file: &File, offset: u64, len: u64) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    // SAFETY: a valid descriptor and in-range arguments; no memory is shared.
    let r = unsafe { libc::fallocate(file.as_raw_fd(), 0, offset as i64, len as i64) };
    if r == 0 {
        return Ok(());
    }
    let err = std::io::Error::last_os_error();
    if err.raw_os_error() == Some(libc::EOPNOTSUPP) {
        return file.set_len(offset + len);
    }
    Err(err)
}

#[cfg(not(target_os = "linux"))]
fn reserve(file: &File, offset: u64, len: u64) -> std::io::Result<()> {
    file.set_len(offset + len)
}

#[cfg(unix)]
fn filesystem_available(dir: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let probe = existing_ancestor(dir)?;
    let path = std::ffi::CString::new(probe.as_os_str().as_bytes()).ok()?;
    // SAFETY: `stat` is plain data written by the call; the path is a valid C string.
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        (libc::statvfs(path.as_ptr(), &mut stat) == 0)
            .then(|| stat.f_bavail as u64 * stat.f_frsize as u64)
    }
}

#[cfg(not(unix))]
fn filesystem_available(_: &Path) -> Option<u64> {
    None
}

fn existing_ancestor(dir: &Path) -> Option<&Path> {
    dir.ancestors().find(|p| p.exists())
}

/// Whether `dir` (or its nearest existing ancestor) is on `tmpfs`.
#[cfg(target_os = "linux")]
pub(super) fn is_tmpfs(dir: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    const TMPFS_MAGIC: i64 = 0x0102_1994;
    let Some(probe) = existing_ancestor(dir) else {
        return false;
    };
    let Ok(path) = std::ffi::CString::new(probe.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: `stat` is plain data written by the call; the path is a valid C string.
    unsafe {
        let mut stat: libc::statfs = std::mem::zeroed();
        libc::statfs(path.as_ptr(), &mut stat) == 0 && stat.f_type as i64 == TMPFS_MAGIC
    }
}

#[cfg(not(target_os = "linux"))]
pub(super) fn is_tmpfs(_: &Path) -> bool {
    false
}
