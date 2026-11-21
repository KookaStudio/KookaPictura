use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};

use super::stderr;
use crate::diagnostic_logging::DRAIN_LIMIT;

const CAPACITY: usize = 128;
const NOTICE_CAPACITY: usize = 16;

static NEXT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
struct LostRows {
    ranges: Vec<(Arc<str>, u64, u64)>,
    more_on_stderr: bool,
    metadata_full: bool,
}

impl LostRows {
    fn add(&mut self, path: Arc<str>, id: u64) {
        if let Some((old_path, first, last)) = self.ranges.last_mut() {
            if *old_path == path && last.checked_add(1) == Some(id) {
                *last = id;
                return;
            }
            if *old_path == path && id.checked_add(1) == Some(*first) {
                *first = id;
                return;
            }
        }
        if self.ranges.len() == NOTICE_CAPACITY {
            self.more_on_stderr = true;
            stderr(&loss(&path, id, id));
        } else {
            self.ranges.push((path, id, id));
        }
    }

    fn take(&mut self) -> String {
        let mut messages: Vec<String> = self
            .ranges
            .drain(..)
            .map(|(path, first, last)| loss(&path, first, last))
            .collect();
        if std::mem::take(&mut self.more_on_stderr) {
            messages.push("Additional History Log write failures were reported to stderr.".into());
        }
        if std::mem::take(&mut self.metadata_full) {
            messages.push("History Log: the document's pending history buffer is full.".into());
        }
        messages.join("\n")
    }
}

fn loss(path: &str, first: u64, last: u64) -> String {
    let range = if first == last {
        first.to_string()
    } else {
        format!("{first}–{last}")
    };
    format!("History Log: could not confirm text rows {range} in {path:?}.")
}

struct Row {
    path: Arc<str>,
    id: u64,
    text: String,
}

type Pending = Arc<Mutex<BTreeMap<u64, Arc<str>>>>;

fn text_destination(file: &mut std::fs::File) -> std::io::Result<()> {
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::other(
            "History Log destination is not a regular file",
        ));
    }
    let mut buffer = [0u8; 8192];
    let count = file.read(&mut buffer)?;
    let bytes = &buffer[..count];
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let head = &bytes[start..];
    if head.starts_with(b"<svg")
        || (head.starts_with(b"<?xml") && head.windows(4).any(|w| w == b"<svg"))
        || head.starts_with(b"P3\n")
        || head.starts_with(b"P3 ")
    {
        return Err(std::io::Error::other("History Log destination is an image"));
    }
    if bytes
        .iter()
        .any(|b| *b < 0x20 && !matches!(*b, b'\n' | b'\r' | b'\t'))
        || std::str::from_utf8(bytes).is_err_and(|error| error.error_len().is_some())
    {
        return Err(std::io::Error::other("History Log destination is binary"));
    }
    Ok(())
}

pub(super) fn is_text_destination(path: &str) -> bool {
    match std::fs::metadata(path) {
        Ok(metadata) if !metadata.is_file() => return false,
        Err(error) => return error.kind() == std::io::ErrorKind::NotFound,
        _ => {}
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY);
    }
    match options.open(path) {
        Ok(mut file) => text_destination(&mut file).is_ok(),
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
    }
}

struct Worker {
    sender: SyncSender<Row>,
    pending: Pending,
    done: Receiver<()>,
    cancelled: Arc<AtomicBool>,
}

#[derive(Default)]
pub(super) struct Output {
    worker: Option<Worker>,
    lost: Arc<Mutex<LostRows>>,
    stalled: Arc<AtomicBool>,
}

impl Output {
    fn start(&mut self) -> std::io::Result<()> {
        let (sender, receiver) = mpsc::sync_channel::<Row>(CAPACITY);
        let (finished, done) = mpsc::channel();
        let pending = Pending::default();
        let worker_pending = pending.clone();
        let lost = self.lost.clone();
        let stalled = self.stalled.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = cancelled.clone();
        std::thread::Builder::new()
            .name("pictura-history-log".into())
            .spawn(move || {
                for row in receiver {
                    while stalled.load(Ordering::Relaxed)
                        && !worker_cancelled.load(Ordering::Acquire)
                    {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    if worker_cancelled.load(Ordering::Acquire) {
                        break;
                    }
                    let result = OpenOptions::new()
                        .create(true)
                        .read(true)
                        .append(true)
                        .open(row.path.as_ref())
                        .and_then(|mut file| {
                            text_destination(&mut file)?;
                            let text = format!("{} {}\n", row.id, row.text);
                            file.write_all(text.as_bytes())?;
                            file.flush()
                        });
                    if result.is_err() {
                        lost.lock()
                            .expect("History Log failures")
                            .add(row.path, row.id);
                    }
                    worker_pending
                        .lock()
                        .expect("History Log pending rows")
                        .remove(&row.id);
                }
                let _ = finished.send(());
            })?;
        self.worker = Some(Worker {
            sender,
            pending,
            done,
            cancelled,
        });
        Ok(())
    }

    pub(super) fn append(&mut self, path: &str, text: &str) {
        let id = NEXT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path: Arc<str> = Arc::from(path);
        if self.worker.is_none() && self.start().is_err() {
            self.lost
                .lock()
                .expect("History Log failures")
                .add(path, id);
            return;
        }
        let worker = self.worker.as_ref().expect("started History Log writer");
        let mut pending = worker.pending.lock().expect("History Log pending rows");
        pending.insert(id, path.clone());
        let row = Row {
            path,
            id,
            text: text.to_owned(),
        };
        if let Err(mpsc::TrySendError::Full(row) | mpsc::TrySendError::Disconnected(row)) =
            worker.sender.try_send(row)
        {
            pending.remove(&id);
            drop(pending);
            self.lost
                .lock()
                .expect("History Log failures")
                .add(row.path, id);
        }
    }

    pub(super) fn metadata_full(&mut self) {
        self.lost
            .lock()
            .expect("History Log failures")
            .metadata_full = true;
    }

    pub(super) fn take_notice(&mut self) -> String {
        self.lost.lock().expect("History Log failures").take()
    }

    pub(super) fn shutdown(&mut self) {
        if let Some(Worker {
            sender,
            pending,
            done,
            cancelled,
        }) = self.worker.take()
        {
            drop(sender);
            if done.recv_timeout(DRAIN_LIMIT).is_err() {
                cancelled.store(true, Ordering::Release);
                let outstanding =
                    std::mem::take(&mut *pending.lock().expect("History Log pending rows"));
                let mut lost = self.lost.lock().expect("History Log failures");
                for (id, path) in outstanding {
                    lost.add(path, id);
                }
            }
        }
        let notice = self.take_notice();
        if !notice.is_empty() {
            stderr(&notice);
        }
        self.stalled.store(false, Ordering::Relaxed);
    }

    pub(super) fn stall_for_test(&mut self, stalled: bool) {
        self.stalled.store(stalled, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_destination_checks_the_open_file() {
        let root =
            std::env::temp_dir().join(format!("pictura-history-{}", super::super::unix_seconds()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("history.txt");
        std::fs::write(&path, "existing café\n").unwrap();
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .open(&path)
            .unwrap();
        assert!(text_destination(&mut file).is_ok());
        drop(file);
        std::fs::write(&path, b"8BPS\0\x01image").unwrap();
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .open(&path)
            .unwrap();
        assert!(text_destination(&mut file).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"8BPS\0\x01image");
        drop(file);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn text_destination_rejects_fifo_without_opening_it() {
        use std::os::unix::ffi::OsStrExt;

        let path = std::env::temp_dir().join(format!(
            "pictura-history-fifo-{}-{}",
            std::process::id(),
            super::super::unix_seconds()
        ));
        let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(!is_text_destination(path.to_str().unwrap()));
        std::fs::remove_file(path).unwrap();
    }
}
