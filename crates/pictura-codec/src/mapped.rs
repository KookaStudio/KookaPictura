//! A read-only memory map of a file, so a PSD/PSB open decodes from the page
//! cache instead of a heap copy of the whole file (#1273). The kernel can
//! reclaim the mapped pages, so they are not the governor's anonymous memory.
//!
//! ponytail: truncating the file while it is mapped raises `SIGBUS` on the
//! next read past the new end. Pictura's own saves land by atomic rename,
//! which never truncates the file an open has mapped.

use std::path::Path;

/// A file mapped read-only; it dereferences to the file's bytes.
///
/// ponytail: off Unix the file is read into the heap, as before #1273.
pub struct MappedFile {
    #[cfg(unix)]
    ptr: *mut libc::c_void,
    #[cfg(unix)]
    len: usize,
    #[cfg(not(unix))]
    bytes: Vec<u8>,
}

// SAFETY: the mapping is read-only and owned; nothing aliases it mutably.
#[cfg(unix)]
unsafe impl Send for MappedFile {}
// SAFETY: as above; shared reads of immutable memory are sound.
#[cfg(unix)]
unsafe impl Sync for MappedFile {}

#[cfg(not(unix))]
impl MappedFile {
    /// Read `path` whole.
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        std::fs::read(path).map(|bytes| Self { bytes })
    }
}

#[cfg(not(unix))]
impl std::ops::Deref for MappedFile {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(unix)]
impl MappedFile {
    /// Map `path`. An empty file maps nothing and reads as an empty slice.
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        use std::os::fd::AsRawFd;
        let file = std::fs::File::open(path)?;
        let len = usize::try_from(file.metadata()?.len())
            .map_err(|_| std::io::Error::other("file too large to map"))?;
        if len == 0 {
            return Ok(Self {
                ptr: std::ptr::null_mut(),
                len: 0,
            });
        }
        // SAFETY: a fresh read-only private mapping of an open descriptor;
        // the result is checked against MAP_FAILED before use.
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                file.as_raw_fd(),
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: advice on the mapping just created; failure is harmless.
        unsafe {
            libc::madvise(ptr, len, libc::MADV_SEQUENTIAL);
        }
        Ok(Self { ptr, len })
    }
}

#[cfg(unix)]
impl std::ops::Deref for MappedFile {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        if self.len == 0 {
            return &[];
        }
        // SAFETY: `ptr` maps `len` readable bytes for the lifetime of `self`.
        unsafe { std::slice::from_raw_parts(self.ptr as *const u8, self.len) }
    }
}

#[cfg(unix)]
impl Drop for MappedFile {
    fn drop(&mut self) {
        if self.len != 0 {
            // SAFETY: unmapping exactly the region `open` mapped, once.
            unsafe {
                libc::munmap(self.ptr, self.len);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::Document;

    #[test]
    fn a_mapped_file_reads_as_its_bytes() {
        let dir = std::env::temp_dir().join(format!("pictura-mapped-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let doc = Document::from_rgba("Layer 0", 5, 3, &[90u8; 5 * 3 * 4]);
        let bytes = crate::write_psd(&doc).expect("write");
        let psd = dir.join("doc.psd");
        std::fs::write(&psd, &bytes).unwrap();
        let empty = dir.join("empty");
        std::fs::write(&empty, b"").unwrap();

        let mapped = MappedFile::open(&psd).expect("map");
        assert_eq!(&mapped[..], &bytes[..]);
        assert_eq!(
            crate::read_psd(&mapped).expect("mapped read"),
            crate::read_psd(&bytes).expect("heap read")
        );
        assert!(MappedFile::open(&empty).expect("empty").is_empty());
        assert!(MappedFile::open(dir.join("missing")).is_err());
        drop(mapped);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
