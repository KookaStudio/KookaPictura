use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::cxxqt_object::helpers::safe_io::{write_atomically, write_durable, DurableWriteError};

pub(super) const BATCH_BYTES: usize = 4 * 1024 * 1024;
pub(super) const SEGMENT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RECORD: usize = 32 * 1024 * 1024;
const BATCH_DELAY: Duration = Duration::from_secs(2);
const HEADER: usize = 32;

#[derive(Default)]
pub(super) struct Batch {
    deadline: Option<Instant>,
    bytes: usize,
}

impl Batch {
    pub(super) fn push(&mut self, now: Instant, bytes: usize) {
        self.deadline.get_or_insert(now + BATCH_DELAY);
        self.bytes = self.bytes.saturating_add(bytes);
    }

    pub(super) fn due(&self, now: Instant) -> bool {
        self.bytes >= BATCH_BYTES || self.deadline.is_some_and(|deadline| now >= deadline)
    }

    pub(super) fn remaining(&self, now: Instant) -> Option<Duration> {
        self.deadline
            .map(|deadline| deadline.saturating_duration_since(now))
    }
}

pub(super) fn checksum(data: &[u8]) -> u32 {
    const POLYNOMIAL: u32 = 0xedb88320;

    const fn table() -> [u32; 256] {
        let mut table = [0; 256];
        let mut i = 0;
        while i < table.len() {
            let mut crc = i as u32;
            let mut bit = 0;
            while bit < 8 {
                crc = (crc >> 1) ^ (POLYNOMIAL & 0u32.wrapping_sub(crc & 1));
                bit += 1;
            }
            table[i] = crc;
            i += 1;
        }
        table
    }
    const TABLE: [u32; 256] = table();

    let mut crc = u32::MAX;
    for &byte in data {
        crc = (crc >> 8) ^ TABLE[((crc as u8) ^ byte) as usize];
    }
    !crc
}

pub(super) fn frame_record(chain: u64, sequence: u64, payload: &[u8]) -> io::Result<Vec<u8>> {
    if payload.len() > MAX_RECORD || sequence == 0 {
        return Err(io::Error::other("invalid journal record size or sequence"));
    }
    let mut record = Vec::with_capacity(HEADER + payload.len() + 4);
    record.extend_from_slice(b"KPJ1");
    record.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    record.extend_from_slice(&chain.to_le_bytes());
    record.extend_from_slice(&sequence.to_le_bytes());
    record.extend_from_slice(&(sequence - 1).to_le_bytes());
    record.extend_from_slice(payload);
    let crc = checksum(&record);
    record.extend_from_slice(&crc.to_le_bytes());
    Ok(record)
}

pub(super) fn append_batch(path: &Path, bytes: &[u8], fail: bool) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).open(path)?;
    let length = file.metadata()?.len();
    let result = file.write_all(bytes).and_then(|()| {
        if fail {
            return Err(io::Error::other("journal sync fault"));
        }
        file.sync_data()
    });
    if result.is_err() {
        file.set_len(length)?;
        file.sync_data()?;
    }
    result
}

/// Returns the last fully applied sequence and whether recovery stopped at a bad tail.
pub(super) fn replay_records(
    path: &Path,
    chain: u64,
    checkpoint: u64,
    mut apply: impl FnMut(u64, &[u8]) -> io::Result<()>,
) -> io::Result<(u64, bool)> {
    let mut reader = io::BufReader::new(fs::File::open(path)?);
    let mut header = [0; HEADER];
    let mut sequence = checkpoint;
    loop {
        if reader.read(&mut header[..1])? == 0 {
            return Ok((sequence, false));
        }
        match reader.read_exact(&mut header[1..]) {
            Ok(()) => (),
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                return Ok((sequence, true))
            }
            Err(error) => return Err(error),
        }
        let size = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
        let record_chain = u64::from_le_bytes(header[8..16].try_into().unwrap());
        let next = u64::from_le_bytes(header[16..24].try_into().unwrap());
        let predecessor = u64::from_le_bytes(header[24..32].try_into().unwrap());
        if &header[..4] != b"KPJ1"
            || size > MAX_RECORD
            || chain != record_chain
            || sequence.checked_add(1) != Some(next)
            || predecessor != sequence
        {
            return Ok((sequence, true));
        }
        let mut record = Vec::with_capacity(HEADER + size + 4);
        record.extend_from_slice(&header);
        record.resize(HEADER + size + 4, 0);
        match reader.read_exact(&mut record[HEADER..]) {
            Ok(()) => (),
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                return Ok((sequence, true))
            }
            Err(error) => return Err(error),
        }
        let split = record.len() - 4;
        let crc = u32::from_le_bytes(record[split..].try_into().unwrap());
        if checksum(&record[..split]) != crc {
            return Ok((sequence, true));
        }
        match apply(next, &record[HEADER..split]) {
            Ok(()) => sequence = next,
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                return Ok((sequence, true))
            }
            Err(error) => return Err(error),
        }
    }
}

/// The current generation must remain available if publication reaches rename
/// but cannot confirm durability.
pub(super) fn publish_manifest(
    path: &Path,
    bytes: &[u8],
    before_rename: impl FnOnce(&Path) -> io::Result<()>,
    sync_parent: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<(), DurableWriteError> {
    before_rename(path).map_err(DurableWriteError::BeforeRename)?;
    write_durable(path, bytes, sync_parent)
}

pub(super) fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let path = path
        .to_str()
        .ok_or_else(|| io::Error::other("invalid recovery path"))?;
    write_atomically(path, bytes, true)
}
