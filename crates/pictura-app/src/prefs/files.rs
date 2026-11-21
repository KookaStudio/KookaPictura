use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const MAX_BYTES: u64 = 1024 * 1024;

pub(super) fn timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub(super) fn unique() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        timestamp(),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

pub(super) fn create_dir(path: &Path) -> io::Result<()> {
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

fn private_options() -> OpenOptions {
    let options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = options;
        options.mode(0o600);
        options
    }
    #[cfg(not(unix))]
    {
        options
    }
}

pub(super) fn lock_file(path: &Path) -> io::Result<File> {
    private_options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
}

pub(super) fn read(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "settings file exceeds size limit",
        ));
    }
    Ok(bytes)
}

pub(super) fn sync_dir(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

pub(super) fn parent(path: &Path) -> &Path {
    path.parent().expect("preference paths have a parent")
}

pub(super) fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut bytes = bytes;
    replace_from_reader(path, &mut bytes)
}

fn replace_from_reader(path: &Path, source: &mut impl Read) -> io::Result<()> {
    let dir = parent(path);
    let temp = dir.join(format!(".pictura-{}.tmp", unique()));
    let result = (|| {
        let mut file = private_options().create_new(true).write(true).open(&temp)?;
        io::copy(source, &mut file)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)?;
        sync_dir(dir)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

pub(super) fn quarantine(path: &Path) -> io::Result<PathBuf> {
    let target = parent(path).join(format!("prefs.toml.corrupt-{}", unique()));
    replace_from_reader(&target, &mut File::open(path)?)?;
    Ok(target)
}

pub(super) fn remove(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => sync_dir(parent(path)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub(super) fn rename(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)?;
    sync_dir(parent(to))?;
    if parent(from) != parent(to) {
        sync_dir(parent(from))?;
    }
    Ok(())
}
