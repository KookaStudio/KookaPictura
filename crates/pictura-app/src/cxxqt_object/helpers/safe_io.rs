//! File writes and reads at the edge of untrusted input (#844).
//!
//! A save lands through a sibling temp file with an unguessable name, created
//! fresh (never through an existing file or symlink), then renamed over the
//! target. A read of a file the user picked is capped before it is buffered.

use std::io::{Read as _, Write as _};
use std::path::Path;

/// Write `bytes` to `path` atomically: a new temp file in the same directory
/// (synced first when `sync`), renamed into place. The temp file is removed
/// on any failure.
pub(in crate::cxxqt_object) fn write_atomically(
    path: &str,
    bytes: &[u8],
    sync: bool,
) -> std::io::Result<()> {
    let tmp = format!(
        "{path}.{:016x}.tmp",
        u64::from_le_bytes(pictura_codec::os_random())
    );
    // `create_new` is O_CREAT|O_EXCL: it fails on any existing entry,
    // including a symlink planted at the predictable-looking name.
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .and_then(|mut file| {
            file.write_all(bytes)?;
            if sync {
                file.sync_all()?;
            }
            Ok(())
        });
    if let Err(e) = written.and_then(|()| std::fs::rename(&tmp, path)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

/// The bytes of `path`, or `None` when it cannot be read or holds more than
/// `limit` bytes.
pub(in crate::cxxqt_object) fn read_capped(path: impl AsRef<Path>, limit: u64) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > limit {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= limit).then_some(bytes)
}

/// The largest raster file an import reads: no compressed image larger than
/// its own decoded budget is plausible.
pub(in crate::cxxqt_object) fn import_file_limit() -> u64 {
    pictura_codec::ImageBudget::default().max_alloc_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "pictura-safe-io-{name}-{}-{:x}",
            std::process::id(),
            u64::from_le_bytes(pictura_codec::os_random())
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn leftovers(dir: &Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp"))
            .collect()
    }

    #[test]
    fn an_atomic_write_replaces_the_target_and_leaves_no_temp_file() {
        let dir = scratch("replace");
        let path = dir.join("out.psd");
        std::fs::write(&path, b"old").unwrap();
        write_atomically(path.to_str().unwrap(), b"new", true).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert!(leftovers(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_planted_temp_symlink_is_not_followed() {
        // The temp name was `<path>.tmp`, so a symlink planted there redirected
        // the write to its target (#844 M6).
        let dir = scratch("symlink");
        let victim = dir.join("victim");
        std::fs::write(&victim, b"keep").unwrap();
        let path = dir.join("out.psd");
        std::os::unix::fs::symlink(&victim, format!("{}.tmp", path.display())).unwrap();
        write_atomically(path.to_str().unwrap(), b"new", false).unwrap();
        assert_eq!(std::fs::read(&victim).unwrap(), b"keep");
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn temp_names_are_unpredictable() {
        let a: [u8; 8] = pictura_codec::os_random();
        let b: [u8; 8] = pictura_codec::os_random();
        assert_ne!(a, b);
    }

    #[test]
    fn a_capped_read_refuses_a_larger_file() {
        let dir = scratch("capped");
        let path = dir.join("big.png");
        std::fs::write(&path, [0u8; 100]).unwrap();
        assert_eq!(read_capped(&path, 100).map(|b| b.len()), Some(100));
        assert_eq!(read_capped(&path, 99), None);
        assert_eq!(read_capped(dir.join("missing"), 100), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
