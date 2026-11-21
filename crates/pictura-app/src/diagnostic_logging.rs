use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::{util::SubscriberInitExt, Layer};

#[cfg(test)]
mod tests;

pub(crate) const DRAIN_LIMIT: Duration = Duration::from_secs(2);
const QUEUE_CAPACITY: usize = 1024;
const SECONDS_PER_DAY: i64 = 86_400;

static FILE: OnceLock<FileQueue> = OnceLock::new();

pub(crate) fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() as i64)
}

pub(crate) fn stderr(message: &str) {
    let _ = writeln!(io::stderr().lock(), "{message}");
}

fn launch_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("{}-{time}-{sequence}", std::process::id())
}

fn day_prefix(time: i64) -> String {
    let (year, month, day, ..) = pictura_core::utc_date_time(time);
    format!("{year:04}-{month:02}-{day:02}")
}

fn prepare_directory(path: &Path) -> io::Result<()> {
    std::fs::create_dir_all(path)
}

struct DailyFiles {
    path: PathBuf,
    launch: String,
    current: Option<(String, File)>,
    created: HashSet<String>,
}

impl DailyFiles {
    fn new(path: PathBuf, launch: String) -> Self {
        Self {
            path,
            launch,
            current: None,
            created: HashSet::new(),
        }
    }

    fn rotate(&mut self, date: String, time: i64) -> io::Result<()> {
        let oldest = day_prefix(time - 6 * SECONDS_PER_DAY);
        for entry in std::fs::read_dir(&self.path)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some((day, launch)) = name.to_str().and_then(|name| name.split_once("-pictura-"))
            else {
                continue;
            };
            if day.len() == 10
                && day
                    .as_bytes()
                    .iter()
                    .all(|b| b.is_ascii_digit() || *b == b'-')
                && day < oldest.as_str()
                && launch.ends_with(".jsonl")
                && entry.file_type()?.is_file()
            {
                match std::fs::remove_file(entry.path()) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
        }
        let path = self
            .path
            .join(format!("{date}-pictura-{}.jsonl", self.launch));
        let mut options = OpenOptions::new();
        options.append(true);
        if self.created.contains(&date) {
            options.create(true);
        } else {
            options.create_new(true);
        }
        let file = options.open(path)?;
        self.created.insert(date.clone());
        self.current = Some((date, file));
        Ok(())
    }

    fn write_at(&mut self, time: i64, record: &[u8]) -> io::Result<()> {
        let date = day_prefix(time);
        if self
            .current
            .as_ref()
            .is_none_or(|(previous, _)| *previous != date)
        {
            self.rotate(date, time)?;
        }
        let (_, file) = self
            .current
            .as_mut()
            .expect("rotated to an open diagnostic file");
        file.write_all(record)?;
        file.flush()
    }
}

struct FileQueue {
    sender: Mutex<Option<SyncSender<Vec<u8>>>>,
    completed: Mutex<Receiver<bool>>,
    failed: Arc<AtomicBool>,
}

impl FileQueue {
    fn spawn(
        capacity: usize,
        mut write: impl FnMut(&[u8]) -> io::Result<()> + Send + 'static,
    ) -> io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(capacity);
        let (done, completed) = mpsc::channel();
        let failed = Arc::new(AtomicBool::new(false));
        let worker_failed = failed.clone();
        std::thread::Builder::new()
            .name("pictura-diagnostics".into())
            .spawn(move || {
                for record in receiver {
                    if write(&record).is_err() {
                        worker_failed.store(true, Ordering::Release);
                        stderr("pictura: diagnostic file sink failed and its drain is incomplete");
                        let _ = done.send(false);
                        return;
                    }
                }
                let _ = done.send(true);
            })?;
        Ok(Self {
            sender: Mutex::new(Some(sender)),
            completed: Mutex::new(completed),
            failed,
        })
    }

    fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }

    fn enqueue(&self, line: Vec<u8>) -> bool {
        !self.failed()
            && self.sender.lock().ok().is_some_and(|slot| {
                slot.as_ref()
                    .is_some_and(|sender| sender.try_send(line).is_ok())
            })
    }

    fn take_completion(&self, timeout: Duration) -> Option<bool> {
        self.completed.lock().ok()?.recv_timeout(timeout).ok()
    }

    fn shutdown(&self) {
        if self
            .sender
            .lock()
            .expect("diagnostic sender lock")
            .take()
            .is_none()
        {
            return;
        }
        if self.take_completion(DRAIN_LIMIT).is_none() {
            stderr("pictura: diagnostic drain incomplete after two seconds");
        }
    }
}

#[derive(Default)]
struct Fields {
    kind: Option<String>,
    category: Option<String>,
    dependency_target: Option<String>,
    fatal: bool,
    success: Option<bool>,
}

fn identifier(value: &str) -> Option<String> {
    (!value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte)))
    .then(|| value.to_owned())
}

impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "event" => self.kind = identifier(value),
            "category" => self.category = identifier(value),
            "log.target" => self.dependency_target = identifier(value),
            _ => {}
        }
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        match field.name() {
            "fatal" => self.fatal = value,
            "success" => self.success = Some(value),
            _ => {}
        }
    }

    fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}
}

struct DiagnosticLayer {
    file: Option<&'static FileQueue>,
    terminal: bool,
    filter: Targets,
    launch: String,
}

impl<S: Subscriber> Layer<S> for DiagnosticLayer {
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let meta = event.metadata();
        let mut fields = Fields::default();
        event.record(&mut fields);
        let (source, kind, target) = match meta.target() {
            "pictura::qt" => ("qt", "qt.message".to_owned(), "pictura::qt".to_owned()),
            target if target.starts_with("pictura::") => {
                let Some(kind) = fields.kind else {
                    return;
                };
                ("rust", kind, target.to_owned())
            }
            target => {
                let Some(target) = fields.dependency_target.or_else(|| identifier(target)) else {
                    return;
                };
                ("dependency", "dependency.log".to_owned(), target)
            }
        };
        if !self.filter.would_enable(&target, meta.level()) {
            return;
        }
        let mut record = Map::from_iter([
            (
                "timestamp".to_owned(),
                json!(pictura_core::utc_timestamp(unix_seconds())),
            ),
            ("level".to_owned(), json!(meta.level().as_str())),
            ("source".to_owned(), json!(source)),
            ("event".to_owned(), json!(kind)),
            ("target".to_owned(), json!(target)),
            ("launch".to_owned(), json!(self.launch)),
        ]);
        if source == "qt" {
            record.insert(
                "category".to_owned(),
                json!(fields.category.unwrap_or_else(|| "other".into())),
            );
            record.insert("fatal".to_owned(), json!(fields.fatal));
        } else if let ("rust", Some(success)) = (source, fields.success) {
            record.insert("success".to_owned(), json!(success));
        }
        let line = Value::Object(record).to_string();
        if source != "qt" && (self.terminal || self.file.is_none_or(FileQueue::failed)) {
            stderr(&line);
        }
        if let Some(file) = self.file {
            let mut bytes = line.into_bytes();
            bytes.push(b'\n');
            file.enqueue(bytes);
        }
    }
}

pub(crate) fn start(path: &str, stall_for_test: bool) {
    let launch = launch_id();
    if !path.is_empty() {
        let root = PathBuf::from(path);
        match prepare_directory(&root).and_then(|()| {
            let mut output = DailyFiles::new(root, launch.clone());
            let now = unix_seconds();
            output.rotate(day_prefix(now), now)?;
            FileQueue::spawn(QUEUE_CAPACITY, move |line| {
                if stall_for_test {
                    std::thread::sleep(DRAIN_LIMIT.saturating_mul(3));
                }
                output.write_at(unix_seconds(), line)
            })
        }) {
            Ok(file) => {
                let _ = FILE.set(file);
            }
            Err(_) => stderr("pictura: diagnostic file sink unavailable, using stderr"),
        }
    }
    let mut filter: Targets = std::env::var("RUST_LOG")
        .ok()
        .filter(|rules| !rules.trim().is_empty())
        .and_then(|rules| rules.trim().parse().ok())
        .unwrap_or_default();
    if filter.default_level().is_none() {
        filter = filter.with_default(LevelFilter::INFO);
    }
    let layer = DiagnosticLayer {
        file: FILE.get(),
        terminal: io::stderr().is_terminal(),
        filter,
        launch,
    };
    if tracing_subscriber::registry()
        .with(layer)
        .try_init()
        .is_err()
    {
        if let Some(file) = FILE.get() {
            file.shutdown();
        }
        stderr("pictura: could not install the diagnostic subscriber");
    }
}

pub(crate) fn shutdown() {
    if let Some(file) = FILE.get() {
        file.shutdown();
    }
}

pub(crate) fn qt(level: i32, category: &str, fatal: bool) {
    match level {
        0 => tracing::event!(target: "pictura::qt", Level::DEBUG, category, fatal),
        1 => tracing::event!(target: "pictura::qt", Level::INFO, category, fatal),
        2 => tracing::event!(target: "pictura::qt", Level::WARN, category, fatal),
        _ => tracing::event!(target: "pictura::qt", Level::ERROR, category, fatal),
    }
}

pub(crate) fn outcome(event: &'static str, success: bool) {
    if success {
        tracing::event!(target: "pictura::app", Level::INFO, event, success);
    } else {
        tracing::event!(target: "pictura::app", Level::WARN, event, success);
    }
}
