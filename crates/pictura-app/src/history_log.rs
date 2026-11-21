use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use pictura_codec::xmp::{patch_photoshop_history, photoshop_history};
use pictura_core::Document;

mod text;

use crate::diagnostic_logging::{stderr, unix_seconds};

const MAX_PENDING_BYTES: usize = 1 << 20;
pub(crate) const APPEND_OMITTED: &str =
    "History Log: the current image was saved without new history entries.";
pub(crate) const REBASE_FAILED: &str =
    "History Log: saved entries could not be merged into the current document and remain available for retry.";

#[derive(Default)]
struct Settings {
    enabled: bool,
    destination: i32,
    verbosity: i32,
    path: String,
}

#[derive(Default)]
struct Runtime {
    settings: Settings,
    text: text::Output,
}

fn runtime() -> &'static Mutex<Runtime> {
    static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();
    RUNTIME.get_or_init(|| Mutex::new(Runtime::default()))
}

pub(crate) fn configure(enabled: bool, destination: i32, verbosity: i32, path: String) {
    let destination = destination.clamp(0, 2);
    let verbosity = verbosity.clamp(0, 2);
    runtime().lock().expect("History Log settings").settings = Settings {
        enabled: enabled && (destination == 0 || !path.is_empty()),
        destination,
        verbosity,
        path,
    };
}

pub(crate) fn lifecycle(action: &str) {
    record(None, action, false, None, false);
}

pub(crate) fn close(name: &str) {
    record(Some(name), "Close", false, None, false);
}

pub(crate) fn take_notice() -> String {
    runtime()
        .lock()
        .expect("History Log settings")
        .text
        .take_notice()
}

pub(crate) fn detailed() -> bool {
    let runtime = runtime().lock().expect("History Log settings");
    runtime.settings.enabled && runtime.settings.verbosity == 2
}

pub(crate) fn destination_conflicts(path: &str) -> bool {
    let log_path = {
        let runtime = runtime().lock().expect("History Log settings");
        let settings = &runtime.settings;
        if !settings.enabled || settings.destination == 0 {
            return false;
        }
        settings.path.clone()
    };
    let log = Path::new(&log_path);
    let image = Path::new(path);
    let resolved = |path: &Path| {
        std::fs::canonicalize(path).ok().or_else(|| {
            let parent = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            Some(std::fs::canonicalize(parent).ok()?.join(path.file_name()?))
        })
    };
    let mut conflict = log == image
        || resolved(log)
            .zip(resolved(image))
            .is_some_and(|(a, b)| a == b);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(a), Ok(b)) = (std::fs::metadata(log), std::fs::metadata(image)) {
            conflict |= a.dev() == b.dev() && a.ino() == b.ino();
        }
    }
    conflict && text::is_text_destination(&log_path)
}

pub(crate) fn filter_detail(kind: &str, values: &[f64]) -> Option<String> {
    if !detailed() {
        return None;
    }
    let spec = pictura_filters::catalogue::descriptor(kind)?;
    let id = pictura_script::command_catalogue::id_for_path(spec.path)?;
    if values.is_empty() {
        return Some(id);
    }
    let mut detail = id;
    let mut remaining = values;
    for parameter in spec.params {
        let (slots, rest) = remaining.split_at(remaining.len().min(parameter.slots()));
        if slots.is_empty() {
            break;
        }
        use std::fmt::Write;
        if let [value] = slots {
            let _ = write!(detail, " {}={value}", parameter.label);
        } else {
            let _ = write!(detail, " {}={slots:?}", parameter.label);
        }
        remaining = rest;
    }
    Some(detail)
}

pub(crate) fn shutdown() {
    let mut output = {
        let mut runtime = runtime().lock().expect("History Log settings");
        std::mem::take(&mut runtime.text)
    };
    output.shutdown();
}

pub(crate) fn stall_for_test(stalled: bool) {
    runtime()
        .lock()
        .expect("History Log settings")
        .text
        .stall_for_test(stalled);
}

fn one_line(value: &str) -> String {
    let mut line = String::new();
    for ch in value.chars().take(4096) {
        if ch.is_control() || matches!(ch, '\u{fffe}' | '\u{ffff}') {
            line.extend(ch.escape_default());
        } else {
            line.push(ch);
        }
    }
    line
}

fn record(
    name: Option<&str>,
    action: &str,
    edit: bool,
    detail: Option<&str>,
    document_metadata: bool,
) -> Option<String> {
    let mut runtime = runtime().lock().expect("History Log settings");
    let Runtime { settings, text } = &mut *runtime;
    if !settings.enabled || (edit && settings.verbosity == 0) {
        return None;
    }
    let stamp = pictura_core::utc_timestamp(unix_seconds());
    let action = one_line(action);
    let mut line = match name {
        Some(name) => format!("{stamp} [{}] {action}", one_line(name)),
        None => format!("{stamp} {action}"),
    };
    if settings.verbosity == 2 {
        if let Some(detail) = detail {
            line.push_str(" (");
            line.push_str(&one_line(detail));
            line.push(')');
        }
    }
    if settings.destination != 0 {
        text.append(&settings.path, &line);
    }
    (document_metadata && settings.destination != 1).then_some(line)
}

#[derive(Default)]
pub(crate) struct DocumentLog {
    name: String,
    pending: VecDeque<String>,
    bytes: usize,
    saved_history: Option<String>,
    history_generation: u64,
    pending_marker: Option<(u64, Option<String>)>,
    warning: bool,
}

pub(crate) struct StagedHistory {
    packet: String,
    text: String,
    count: usize,
    generation: u64,
}

impl StagedHistory {
    pub(crate) fn packet(&self) -> &str {
        &self.packet
    }
}

impl DocumentLog {
    pub(crate) fn set_name(&mut self, name: &str, opened: bool) {
        self.name = name.to_owned();
        if opened {
            let row = record(Some(&self.name), "Open", false, None, true);
            self.push(row);
        }
    }

    pub(crate) fn set_filename(&mut self, path: &str) {
        if let Some(name) = Path::new(path).file_name().and_then(|value| value.to_str()) {
            self.set_name(name, false);
        }
    }

    pub(crate) fn edit(&mut self, label: &str, detail: Option<&str>) {
        let name = if self.name.is_empty() {
            "Untitled"
        } else {
            &self.name
        };
        let row = record(Some(name), label, true, detail, true);
        self.push(row);
    }

    fn push(&mut self, row: Option<String>) {
        if let Some(row) = row {
            let row_bytes = row.len() + 1;
            if self.bytes + row_bytes <= MAX_PENDING_BYTES {
                self.bytes += row_bytes;
                self.pending.push_back(row);
            } else if !self.warning {
                self.warning = true;
                runtime()
                    .lock()
                    .expect("History Log settings")
                    .text
                    .metadata_full();
            }
        }
    }

    pub(crate) fn raw_history_edit(&mut self, before: Option<String>, after: Option<String>) {
        if before != after {
            self.history_generation = self.history_generation.wrapping_add(1);
            self.saved_history = after.clone();
            self.pending_marker = Some((self.history_generation, after));
        }
    }

    pub(crate) fn take_marker(&mut self) -> Option<(u64, Option<String>)> {
        self.pending_marker.take()
    }

    pub(crate) fn restore_history(&mut self, value: Option<String>) {
        self.history_generation = self.history_generation.wrapping_add(1);
        self.saved_history = value;
    }

    pub(crate) fn stage(&self, packet: &str, limit: usize) -> Result<Option<StagedHistory>, ()> {
        if self.pending.is_empty() && self.saved_history.is_none() {
            return Ok(None);
        }
        let existing = photoshop_history(packet);
        if self.pending.is_empty() && self.saved_history == existing {
            return Ok(None);
        }
        let mut text = self.saved_history.clone().or(existing).unwrap_or_default();
        for row in &self.pending {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(row);
        }
        let packet = patch_photoshop_history(packet, &text).ok_or(())?;
        if packet.len() > limit {
            return Err(());
        }
        Ok(Some(StagedHistory {
            packet,
            text,
            count: self.pending.len(),
            generation: self.history_generation,
        }))
    }

    pub(crate) fn stage_bytes(
        &self,
        packet: Option<&[u8]>,
        limit: usize,
    ) -> Result<Option<StagedHistory>, ()> {
        if self.pending.is_empty() && self.saved_history.is_none() {
            return Ok(None);
        }
        let packet = std::str::from_utf8(packet.unwrap_or_default()).map_err(|_| ())?;
        self.stage(packet, limit)
    }

    pub(crate) fn stage_document(&self, doc: &mut Document) -> Result<Option<StagedHistory>, ()> {
        if self.pending.is_empty() && self.saved_history.is_none() {
            return Ok(None);
        }
        let packet = pictura_codec::raster_metadata(doc).xmp;
        let packet = std::str::from_utf8(packet.as_deref().unwrap_or_default()).map_err(|_| ())?;
        let staged = self.stage(packet, usize::MAX)?;
        if let Some(staged) = &staged {
            if staged.packet != packet && !pictura_codec::set_xmp_packet(doc, &staged.packet) {
                return Err(());
            }
        }
        Ok(staged)
    }

    /// Rebase the included prefix against the live packet before retiring it.
    pub(crate) fn acknowledge(&mut self, doc: &mut Document, staged: StagedHistory) -> bool {
        if self.history_generation == staged.generation {
            let packet = pictura_codec::raster_metadata(doc).xmp;
            let Ok(packet) = std::str::from_utf8(packet.as_deref().unwrap_or_default()) else {
                return false;
            };
            let replacement = patch_photoshop_history(packet, &staged.text);
            let Some(replacement) = replacement else {
                return false;
            };
            if packet != replacement && !pictura_codec::set_xmp_packet(doc, &replacement) {
                return false;
            }
            self.saved_history = Some(staged.text);
        }
        for row in self.pending.drain(..staged.count.min(self.pending.len())) {
            self.bytes -= row.len() + 1;
        }
        self.warning = false;
        true
    }
}
