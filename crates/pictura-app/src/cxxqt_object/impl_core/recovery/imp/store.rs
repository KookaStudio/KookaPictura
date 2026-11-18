use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Seek as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

use pictura_codec::print_resources::PrintResourceEdits;
use pictura_core::memory::Reservation;
use pictura_core::{Document, Plane};

use super::journal::{
    append_batch, checksum, frame_record, publish_manifest, replay_records, write_synced, Batch,
    SEGMENT_BYTES,
};
use crate::cxxqt_object::filter_tools::store_planes;
use crate::cxxqt_object::helpers::{safe_io, save_gate};

const MAX_QUEUED_COMMITS: usize = 8;
const MAX_RESOURCES: usize = 64 * 1024 * 1024;
const RECOVERY_FORMAT: u64 = 2;

pub(in crate::cxxqt_object) struct RecoveryAttachment {
    writer: Arc<Writer>,
    id: u32,
    pending: Arc<Mutex<Pending>>,
    status: Arc<AtomicU64>,
    interval: Duration,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::cxxqt_object) struct RecoveryIdentity {
    pub(in crate::cxxqt_object) history: Option<u64>,
    pub(in crate::cxxqt_object) metadata: u64,
    pub(in crate::cxxqt_object) generation: u64,
}

struct Input {
    identity: RecoveryIdentity,
    doc: Document,
    planes: Option<pictura_filters::VpPlanes>,
    print_resources: Arc<PrintResourceEdits>,
    cache: Arc<pictura_codec::EncodeCache>,
    force: bool,
    fault: i32,
    epoch: u64,
    _memory: Reservation<'static>,
}

#[derive(Default)]
struct Pending {
    inputs: Vec<Input>,
    wake_pending: bool,
    name: String,
    path: String,
    next_fault: i32,
}

enum Work {
    Capture {
        id: u32,
        pending: Arc<Mutex<Pending>>,
        status: Arc<AtomicU64>,
        interval: Duration,
    },
    Flush(u32, Arc<AtomicU64>, mpsc::Sender<bool>),
    Retire(u32, Arc<AtomicU64>, u64, Option<mpsc::Sender<bool>>),
    #[cfg(test)]
    Hold(mpsc::Sender<()>, mpsc::Receiver<()>),
}

struct Writer {
    tx: mpsc::Sender<Work>,
}

fn writer_for(directory: &Path) -> Arc<Writer> {
    static WRITERS: OnceLock<Mutex<HashMap<PathBuf, Weak<Writer>>>> = OnceLock::new();
    let mut writers = WRITERS.get_or_init(Mutex::default).lock().unwrap();
    if let Some(writer) = writers.get(directory).and_then(Weak::upgrade) {
        return writer;
    }
    writers.retain(|_, writer| writer.strong_count() != 0);
    let (tx, rx) = mpsc::channel();
    let root = directory.to_owned();
    std::thread::spawn(move || run(root, rx));
    let writer = Arc::new(Writer { tx });
    writers.insert(directory.to_owned(), Arc::downgrade(&writer));
    writer
}

impl RecoveryAttachment {
    pub(super) fn new(directory: &Path, id: u32, name: String, path: String, minutes: u32) -> Self {
        Self {
            writer: writer_for(directory),
            id,
            pending: Arc::new(Mutex::new(Pending {
                name,
                path,
                ..Pending::default()
            })),
            status: Arc::new(AtomicU64::new(0)),
            interval: Duration::from_secs(u64::from(minutes) * 60),
        }
    }

    pub(super) fn update(&mut self, name: String, path: String, minutes: u32) {
        let mut pending = self.pending.lock().unwrap();
        pending.name = name;
        pending.path = path;
        self.interval = Duration::from_secs(u64::from(minutes) * 60);
    }

    pub(in crate::cxxqt_object) fn capture(
        &self,
        doc: &Document,
        planes: Option<&pictura_filters::VpPlanes>,
        print_resources: &Arc<PrintResourceEdits>,
        cache: &Arc<pictura_codec::EncodeCache>,
        identity: RecoveryIdentity,
        force: bool,
    ) {
        let mut pending = self.pending.lock().unwrap();
        let epoch = self.status.fetch_add(2, Ordering::AcqRel).wrapping_add(2) & !1;
        let mut force = force || self.status() != 0;
        if pending.inputs.len() == MAX_QUEUED_COMMITS {
            pending.inputs.clear();
            self.status.fetch_or(1, Ordering::AcqRel);
            force = true;
        }
        let bytes = crate::history::document_bytes(doc) + crate::history::tile_table_bytes(doc);
        let memory = match pictura_core::memory::governor().reserve(bytes, "recovery snapshot") {
            Ok(memory) => memory,
            Err(_) => {
                pending.inputs.clear();
                self.status.fetch_or(1, Ordering::AcqRel);
                return;
            }
        };
        let fault = std::mem::take(&mut pending.next_fault);
        pending.inputs.push(Input {
            identity,
            doc: doc.clone(),
            planes: planes.cloned(),
            print_resources: print_resources.clone(),
            cache: cache.clone(),
            force,
            fault,
            epoch,
            _memory: memory,
        });
        if !pending.wake_pending {
            pending.wake_pending = true;
            let command = Work::Capture {
                id: self.id,
                pending: self.pending.clone(),
                status: self.status.clone(),
                interval: self.interval,
            };
            if self.writer.tx.send(command).is_err() {
                self.status.fetch_or(1, Ordering::AcqRel);
                pending.wake_pending = false;
                pending.inputs.clear();
            }
        }
    }

    pub(super) fn status(&self) -> i32 {
        (self.status.load(Ordering::Acquire) & 1) as i32
    }

    pub(super) fn fail_next(&self, fault: i32) {
        self.pending.lock().unwrap().next_fault = fault;
    }

    pub(super) fn flush(&self) -> bool {
        let (tx, rx) = mpsc::channel();
        self.writer
            .tx
            .send(Work::Flush(self.id, self.status.clone(), tx))
            .is_ok()
            && rx.recv().unwrap_or(false)
    }

    pub(super) fn retire_async(&mut self) -> bool {
        let replacement = {
            let mut old = self.pending.lock().unwrap();
            old.inputs.clear();
            Pending {
                name: old.name.clone(),
                path: old.path.clone(),
                next_fault: std::mem::take(&mut old.next_fault),
                ..Pending::default()
            }
        };
        self.pending = Arc::new(Mutex::new(replacement));
        let epoch = self.status.load(Ordering::Acquire) & !1;
        let sent = self
            .writer
            .tx
            .send(Work::Retire(self.id, self.status.clone(), epoch, None))
            .is_ok();
        if !sent {
            self.status.fetch_or(1, Ordering::AcqRel);
        }
        sent
    }

    pub(in crate::cxxqt_object) fn retire(&self) -> bool {
        self.pending.lock().unwrap().inputs.clear();
        let (tx, rx) = mpsc::channel();
        let epoch = self.status.load(Ordering::Acquire) & !1;
        let success = self
            .writer
            .tx
            .send(Work::Retire(self.id, self.status.clone(), epoch, Some(tx)))
            .is_ok()
            && rx.recv().unwrap_or(false);
        if !success {
            self.status.fetch_or(1, Ordering::AcqRel);
        }
        success
    }
}

struct Generation {
    sequence: u64,
    snapshot: String,
    resources: String,
    journal: String,
}

struct Chain {
    id: u32,
    nonce: u64,
    sequence: u64,
    durable_sequence: u64,
    generation: Option<Generation>,
    uncertain: Vec<Generation>,
    previous: Option<Input>,
    queue: Vec<u8>,
    sync: Batch,
    journal_bytes: u64,
    last_checkpoint: Instant,
    status: Arc<AtomicU64>,
}

impl Chain {
    fn new(id: u32, status: Arc<AtomicU64>) -> Self {
        Self {
            id,
            nonce: u64::from_le_bytes(pictura_codec::os_random()),
            sequence: 0,
            durable_sequence: 0,
            generation: None,
            uncertain: Vec::new(),
            previous: None,
            queue: Vec::new(),
            sync: Batch::default(),
            journal_bytes: 0,
            last_checkpoint: Instant::now(),
            status,
        }
    }

    fn capture(
        &mut self,
        root: &Path,
        name: &str,
        path: &str,
        mut input: Input,
        interval: Duration,
    ) -> io::Result<()> {
        let now = Instant::now();
        let unchanged = self
            .previous
            .as_ref()
            .is_some_and(|old| old.identity == input.identity);
        if unchanged && input.fault == 0 && self.status.load(Ordering::Acquire) & 1 == 0 {
            let checkpoint_current = self
                .generation
                .as_ref()
                .is_some_and(|generation| generation.sequence == self.sequence);
            if !input.force || checkpoint_current {
                return Ok(());
            }
        }
        let next = self
            .sequence
            .checked_add(1)
            .ok_or_else(|| io::Error::other("sequence overflow"))?;
        let before_reads = pictura_core::memory::eviction::unreadable_reads();
        input.print_resources.apply(&mut input.doc);
        let metadata = store_planes(input.planes.as_ref(), &input.doc);
        pictura_codec::set_vanishing_point_data(&mut input.doc, metadata.as_deref());
        if crate::history::unreadable_tiles(&input.doc) != 0 {
            return Err(io::Error::other("unreadable recovery tiles"));
        }
        let delta = if input.force
            || self.status.load(Ordering::Acquire) & 1 != 0
            || now.duration_since(self.last_checkpoint) >= interval
            || self.journal_bytes >= SEGMENT_BYTES
        {
            None
        } else {
            self.previous
                .as_ref()
                .and_then(|old| crate::history::recovery_delta(&old.doc, &input.doc))
        };
        if pictura_core::memory::eviction::unreadable_reads() != before_reads
            && crate::history::unreadable_tiles(&input.doc) != 0
        {
            return Err(io::Error::other("a recovery tile could not be read"));
        }
        if let Some(payload) = delta {
            if payload.is_empty() && input.fault == 0 {
                self.previous = Some(input);
                return Ok(());
            }
            let record = frame_record(self.nonce, next, &payload)?;
            self.sync.push(now, record.len());
            self.queue.extend_from_slice(&record);
            self.sequence = next;
            self.previous = Some(input);
            if self.sync.due(now) || self.previous.as_ref().is_some_and(|i| i.fault == 1) {
                self.flush(root)?;
            }
            return Ok(());
        }
        // Older queued edits are subsumed by this immutable checkpoint. Keep
        // the previous durable generation until the new manifest is durable.
        if let Err(error) = self.checkpoint(root, name, path, &input, next) {
            if matches!(&error, safe_io::DurableWriteError::AfterRename(_)) {
                // Either generation may be named by the durable manifest.
                if let Some(old) = self.generation.take() {
                    self.uncertain.push(old);
                }
                self.previous = None;
                self.queue.clear();
                self.sync = Batch::default();
                self.sequence = self.durable_sequence;
            }
            return Err(error.into());
        }
        self.sequence = next;
        self.durable_sequence = next;
        let epoch = input.epoch;
        self.previous = Some(input);
        self.queue.clear();
        self.sync = Batch::default();
        self.journal_bytes = 0;
        self.last_checkpoint = Instant::now();
        clear_covered_status(&self.status, epoch);
        Ok(())
    }

    fn flush(&mut self, root: &Path) -> io::Result<()> {
        let Some(generation) = self.generation.as_ref() else {
            return Ok(());
        };
        if self.queue.is_empty() {
            return Ok(());
        }
        let fail = self
            .previous
            .as_mut()
            .is_some_and(|input| std::mem::take(&mut input.fault) == 1);
        let result = append_batch(&root.join(&generation.journal), &self.queue, fail);
        self.sync = Batch::default();
        match result {
            Ok(()) => {
                self.journal_bytes += self.queue.len() as u64;
                self.durable_sequence = self.sequence;
                self.queue.clear();
                if let Some(input) = &self.previous {
                    clear_covered_status(&self.status, input.epoch);
                }
                Ok(())
            }
            Err(error) => {
                self.queue.clear();
                self.previous = None;
                self.sequence = self.durable_sequence;
                Err(error)
            }
        }
    }

    fn checkpoint(
        &mut self,
        root: &Path,
        name: &str,
        path: &str,
        input: &Input,
        sequence: u64,
    ) -> Result<(), safe_io::DurableWriteError> {
        use safe_io::DurableWriteError::BeforeRename;

        // A failed entry sync can leave the new directory visible without
        // having made its name durable. Retry the whole path on publication.
        sync_store_entries(
            root,
            input
                .fault
                .checked_sub(2)
                .and_then(|n| usize::try_from(n).ok()),
        )
        .map_err(BeforeRename)?;
        if input.fault == 1 {
            return Err(BeforeRename(io::Error::other("checkpoint sync fault")));
        }
        let ticket = u64::from_le_bytes(pictura_codec::os_random());
        let stem = format!("{}-{ticket:016x}", self.id);
        if input.doc.image_resources.len() > MAX_RESOURCES {
            return Err(BeforeRename(io::Error::other(
                "recovery resources exceed their size limit",
            )));
        }
        let raw = input.doc.image_resources.to_vec();
        let snapshot =
            write_checkpoint(root, &stem, &input.doc, &input.cache).map_err(BeforeRename)?;
        let next = Generation {
            sequence,
            snapshot,
            resources: format!("{stem}.resources"),
            journal: format!("{stem}.journal"),
        };
        let artifacts = [
            root.join(&next.snapshot),
            root.join(&next.resources),
            root.join(&next.journal),
        ];
        let publish = || {
            write_synced(&artifacts[1], &raw)
                .and_then(|()| write_synced(&artifacts[2], &[]))
                .and_then(|()| safe_io::sync_directory(root))
                .map_err(safe_io::DurableWriteError::BeforeRename)?;
            let manifest = serde_json::json!({
                "version": RECOVERY_FORMAT,
                "name": name,
                "originalPath": path,
                "chain": self.nonce,
                "sequence": next.sequence,
                "snapshot": next.snapshot,
                "resources": next.resources,
                "resourceLength": raw.len(),
                "resourceCrc": checksum(&raw),
                "journal": next.journal,
            });
            let manifest = serde_json::to_vec(&manifest)
                .map_err(|error| safe_io::DurableWriteError::BeforeRename(error.into()))?;
            publish_manifest(
                &root.join(format!("{}.json", self.id)),
                &manifest,
                |_| Ok(()),
                |parent| {
                    if input.fault == 100 {
                        return Err(io::Error::other("manifest directory sync fault"));
                    }
                    safe_io::sync_directory(parent)
                },
            )
        };
        if let Err(error) = publish() {
            if matches!(&error, safe_io::DurableWriteError::BeforeRename(_)) {
                for file in &artifacts {
                    let _ = fs::remove_file(file);
                }
            } else {
                self.uncertain.push(next);
            }
            return Err(error);
        }
        if let Some(old) = self.generation.replace(next) {
            self.uncertain.push(old);
        }
        for old in self.uncertain.drain(..) {
            for file in [&old.snapshot, &old.resources, &old.journal] {
                let _ = fs::remove_file(root.join(file));
            }
        }
        Ok(())
    }
}

/// Stream the checkpoint of `source` into a synced file under `root` named
/// from `stem` (#1320); returns the file's name.
pub(super) fn write_checkpoint(
    root: &Path,
    stem: &str,
    source: &Document,
    cache: &pictura_codec::EncodeCache,
) -> io::Result<String> {
    pictura_codec::warm_encode_idents(source);
    let _memory = pictura_core::memory::governor()
        .reserve(save_gate::save_cost(source).working, "recovery checkpoint")
        .map_err(|_| io::Error::other("not enough memory for recovery checkpoint"))?;
    let before_reads = pictura_core::memory::eviction::unreadable_reads();
    let mut doc = source.clone();
    crate::cxxqt_object::impl_core::save_jobs::prepare_native_save(&mut doc, true);
    let mut native = pictura_render::save_view(&doc);
    let mut psb = native.is_psb || native.width > 30_000 || native.height > 30_000;
    loop {
        let suffix = if psb { "psb" } else { "psd" };
        let filename = format!("{stem}.{suffix}");
        // Each attempt starts with the original raw bytes, so the second
        // attempt never retains the generated PSD path.
        native.to_mut().image_resources = source.image_resources.clone();
        pictura_codec::set_recovery_resources(
            native.to_mut(),
            &root.join(&filename).to_string_lossy(),
            psb,
        );
        let path = root.join(&filename);
        let path = path
            .to_str()
            .ok_or_else(|| io::Error::other("invalid recovery path"))?;
        let mut too_large = false;
        let written = safe_io::write_atomically_with(path, true, |out| {
            pictura_codec::write_psd_to(out, &native, psb, Some(cache))
                .map_err(|error| io::Error::other(error.to_string()))?;
            if pictura_core::memory::eviction::unreadable_reads() != before_reads
                && crate::history::unreadable_tiles(&native) != 0
            {
                return Err(io::Error::other("a checkpoint tile could not be read"));
            }
            too_large = !psb && out.stream_position()? >= 2 * 1024 * 1024 * 1024;
            if too_large {
                return Err(io::Error::other("the checkpoint exceeds the PSD size"));
            }
            Ok(())
        });
        if too_large {
            psb = true;
            continue;
        }
        return written.map(|()| filename);
    }
}

fn sync_store_entries(root: &Path, fail_at: Option<usize>) -> io::Result<()> {
    let owners: Vec<_> = root
        .ancestors()
        .skip(1)
        .filter(|path| !path.as_os_str().is_empty())
        .collect();
    for (index, parent) in owners.iter().rev().enumerate() {
        if fail_at == Some(owners.len() - index - 1) {
            return Err(io::Error::other("recovery ancestor sync fault"));
        }
        safe_io::sync_directory(parent)?;
    }
    Ok(())
}

fn clear_covered_status(status: &AtomicU64, epoch: u64) {
    let _ = status.compare_exchange(epoch | 1, epoch, Ordering::AcqRel, Ordering::Acquire);
}

fn mark_error(chain: &Chain, result: io::Result<()>) -> bool {
    if let Err(error) = result {
        eprintln!("pictura: recovery protection delayed: {error}");
        chain.status.fetch_or(1, Ordering::AcqRel);
        return false;
    }
    true
}

fn run(root: PathBuf, receiver: mpsc::Receiver<Work>) {
    let mut docs = HashMap::<u32, Chain>::new();
    loop {
        let now = Instant::now();
        for chain in docs.values_mut().filter(|chain| chain.sync.due(now)) {
            let flushed = chain.flush(&root);
            mark_error(chain, flushed);
        }
        let timeout = docs
            .values()
            .filter_map(|chain| chain.sync.remaining(now))
            .min();
        let command = match timeout {
            Some(timeout) => match receiver.recv_timeout(timeout) {
                Ok(command) => command,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            },
            None => match receiver.recv() {
                Ok(command) => command,
                Err(_) => break,
            },
        };
        match command {
            Work::Capture {
                id,
                pending,
                status,
                interval,
            } => {
                let (inputs, name, path) = {
                    let mut pending = pending.lock().unwrap();
                    pending.wake_pending = false;
                    (
                        std::mem::take(&mut pending.inputs),
                        pending.name.clone(),
                        pending.path.clone(),
                    )
                };
                let chain = docs.entry(id).or_insert_with(|| Chain::new(id, status));
                for input in inputs {
                    let captured = chain.capture(&root, &name, &path, input, interval);
                    mark_error(chain, captured);
                }
            }
            Work::Flush(id, status, reply) => {
                let mut success = status.load(Ordering::Acquire) & 1 == 0;
                if let Some(chain) = docs.get_mut(&id) {
                    let flushed = chain.flush(&root);
                    success &= mark_error(chain, flushed);
                }
                let _ = reply.send(success);
            }
            #[cfg(test)]
            Work::Hold(entered, release) => {
                let _ = entered.send(());
                let _ = release.recv();
            }
            Work::Retire(id, status, epoch, reply) => {
                docs.remove(&id);
                let success = retire(&root, id).is_ok();
                if success {
                    clear_covered_status(&status, epoch);
                } else {
                    status.fetch_or(1, Ordering::AcqRel);
                }
                if let Some(reply) = reply {
                    let _ = reply.send(success);
                }
            }
        }
    }
}

fn retire(root: &Path, id: u32) -> io::Result<()> {
    let manifest = format!("{id}.json");
    match fs::remove_file(root.join(&manifest)) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
        _ => (),
    }
    safe_io::sync_directory(root)?;
    let prefix = format!("{id}-");
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with(&prefix) && entry.file_type()?.is_file()
        {
            fs::remove_file(entry.path())?;
        }
    }
    safe_io::sync_directory(root)
}

fn local_file(root: &Path, value: &serde_json::Value) -> io::Result<PathBuf> {
    let name = value
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(invalid_manifest)?;
    let path = Path::new(name);
    if !matches!(
        path.components().next(),
        Some(std::path::Component::Normal(_))
    ) || path.components().count() != 1
    {
        return Err(invalid_manifest());
    }
    let local = root.join(path);
    if !fs::symlink_metadata(&local)?.file_type().is_file() {
        return Err(invalid_manifest());
    }
    Ok(local)
}

fn invalid_manifest() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid recovery manifest")
}

pub(super) struct Restore {
    pub(super) checkpoint: PathBuf,
    pub(super) resources: Plane<u8>,
    journal: PathBuf,
    chain: u64,
    sequence: u64,
}

impl Restore {
    pub(super) fn read(manifest: &Path) -> io::Result<Self> {
        let root = manifest.parent().ok_or_else(invalid_manifest)?;
        let file = fs::File::open(manifest)?;
        if file.metadata()?.len() > 64 * 1024 {
            return Err(invalid_manifest());
        }
        let value: serde_json::Value = serde_json::from_reader(file)?;
        if value["version"].as_u64() != Some(RECOVERY_FORMAT) {
            return Err(invalid_manifest());
        }
        let resources = local_file(root, &value["resources"])?;
        let raw_len = value["resourceLength"]
            .as_u64()
            .ok_or_else(invalid_manifest)?;
        let crc = value["resourceCrc"].as_u64().ok_or_else(invalid_manifest)?;
        let mut raw = Vec::new();
        if raw_len > MAX_RESOURCES as u64 || fs::metadata(&resources)?.len() != raw_len {
            return Err(invalid_manifest());
        }
        fs::File::open(resources)?
            .take(raw_len + 1)
            .read_to_end(&mut raw)?;
        if raw.len() as u64 != raw_len || u64::from(checksum(&raw)) != crc {
            return Err(io::Error::other(
                "recovery resource sidecar could not be validated",
            ));
        }
        Ok(Self {
            checkpoint: local_file(root, &value["snapshot"])?,
            journal: local_file(root, &value["journal"])?,
            chain: value["chain"].as_u64().ok_or_else(invalid_manifest)?,
            sequence: value["sequence"].as_u64().ok_or_else(invalid_manifest)?,
            resources: raw.into(),
        })
    }

    pub(super) fn replay(&self, doc: &mut Document) -> io::Result<(u64, bool)> {
        replay_records(&self.journal, self.chain, self.sequence, |_, bytes| {
            crate::history::apply_recovery_delta(doc, bytes)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(epoch: u64, force: bool, fault: i32) -> Input {
        let doc = Document::from_rgba("Layer", 2, 2, &[42; 16]);
        Input {
            identity: RecoveryIdentity {
                history: Some(epoch),
                metadata: 0,
                generation: 1,
            },
            doc,
            planes: None,
            print_resources: Arc::default(),
            cache: Arc::new(pictura_codec::EncodeCache::new()),
            force,
            fault,
            epoch,
            _memory: pictura_core::memory::governor()
                .reserve(1, "recovery test")
                .unwrap(),
        }
    }

    #[test]
    fn checkpoint_preserves_unparsed_resources_in_the_sidecar() {
        let root = std::env::temp_dir().join(format!(
            "pictura-resources-{:016x}",
            u64::from_le_bytes(pictura_codec::os_random())
        ));
        fs::create_dir(&root).unwrap();
        let mut checkpoint = input(2, true, 0);
        let record = pictura_codec::frame_image_resource(1086, "", b"foreign");
        let raw = [record.raw.as_slice(), b"unparsed"].concat();
        checkpoint.doc.image_resources = raw.clone().into();
        let mut chain = Chain::new(1, Arc::new(AtomicU64::new(2)));
        chain
            .capture(&root, "test", "", checkpoint, Duration::from_secs(60))
            .unwrap();
        let restore = Restore::read(&root.join("1.json")).unwrap();
        let mut doc = pictura_codec::read_psd(&fs::read(&restore.checkpoint).unwrap()).unwrap();
        assert!(!pictura_codec::document_version(&doc).access.is_read_only());
        doc.image_resources = restore.resources.clone();
        assert_eq!(doc.image_resources.as_ref(), raw.as_slice());
        assert!(pictura_codec::document_version(&doc).access.is_read_only());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn older_completion_cannot_clear_newer_capture_failure() {
        let status = AtomicU64::new(2);
        status.fetch_add(2, Ordering::AcqRel);
        status.fetch_or(1, Ordering::AcqRel);
        clear_covered_status(&status, 2);
        assert_eq!(status.load(Ordering::Acquire), 5);
        clear_covered_status(&status, 4);
        assert_eq!(status.load(Ordering::Acquire), 4);
    }

    #[test]
    fn async_retire_fences_pre_and_post_save_captures() {
        let root = std::env::temp_dir().join(format!(
            "pictura-retire-{:016x}",
            u64::from_le_bytes(pictura_codec::os_random())
        ));
        fs::create_dir(&root).unwrap();
        let mut attachment = RecoveryAttachment::new(&root, 1, "test".into(), "".into(), 10);
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        assert!(attachment
            .writer
            .tx
            .send(Work::Hold(entered_tx, release_rx))
            .is_ok());
        entered_rx.recv().unwrap();
        let doc = Document::from_rgba("Layer", 2, 2, &[42; 16]);
        let cache = Arc::new(pictura_codec::EncodeCache::new());
        let identity = RecoveryIdentity {
            history: Some(1),
            metadata: 0,
            generation: 1,
        };
        let print_resources = Arc::default();
        attachment.capture(&doc, None, &print_resources, &cache, identity, false);
        assert!(attachment.retire_async());
        attachment.capture(
            &doc,
            None,
            &print_resources,
            &cache,
            RecoveryIdentity {
                history: Some(2),
                ..identity
            },
            false,
        );
        release_tx.send(()).unwrap();
        assert!(attachment.flush());
        assert!(root.join("1.json").exists());
        assert!(attachment.retire());
        drop(attachment);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn uncertain_manifest_fences_old_journal_until_new_checkpoint() {
        let root = std::env::temp_dir().join(format!(
            "pictura-fence-{:016x}",
            u64::from_le_bytes(pictura_codec::os_random())
        ));
        fs::create_dir(&root).unwrap();
        let status = Arc::new(AtomicU64::new(2));
        let mut chain = Chain::new(1, status.clone());
        chain
            .capture(
                &root,
                "test",
                "",
                input(2, true, 0),
                Duration::from_secs(60),
            )
            .unwrap();
        let old_journal = root.join(&chain.generation.as_ref().unwrap().journal);
        chain.queue = frame_record(chain.nonce, 2, b"pending").unwrap();
        chain.sync.push(Instant::now(), chain.queue.len());
        chain.sequence = 2;
        chain.previous = Some(input(4, false, 0));

        status.store(6, Ordering::Release);
        let result = chain.capture(
            &root,
            "test",
            "",
            input(6, true, 1),
            Duration::from_secs(60),
        );
        assert!(!mark_error(&chain, result));
        assert!(chain.generation.is_some());
        assert!(!chain.queue.is_empty());
        assert_eq!(fs::metadata(&old_journal).unwrap().len(), 0);

        status.store(9, Ordering::Release);
        let result = chain.capture(
            &root,
            "test",
            "",
            input(8, true, 100),
            Duration::from_secs(60),
        );
        assert!(!mark_error(&chain, result));
        assert!(chain.generation.is_none());
        assert!(chain.queue.is_empty());
        assert!(old_journal.exists());
        assert_eq!(fs::metadata(&old_journal).unwrap().len(), 0);
        assert_eq!(status.load(Ordering::Acquire) & 1, 1);
        let candidates: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(candidates.len(), 7);

        status.store(11, Ordering::Release);
        chain
            .capture(
                &root,
                "test",
                "",
                input(10, false, 0),
                Duration::from_secs(60),
            )
            .unwrap();
        assert_eq!(status.load(Ordering::Acquire) & 1, 0);
        let restore = Restore::read(&root.join("1.json")).unwrap();
        let mut recovered =
            pictura_codec::read_psd(&fs::read(&restore.checkpoint).unwrap()).unwrap();
        assert!(!restore.replay(&mut recovered).unwrap().1);
        for file in candidates {
            if file != root.join("1.json") {
                assert!(!file.exists());
            }
        }
        assert_eq!(fs::read_dir(&root).unwrap().count(), 4);
        fs::remove_dir_all(root).unwrap();
    }
}
