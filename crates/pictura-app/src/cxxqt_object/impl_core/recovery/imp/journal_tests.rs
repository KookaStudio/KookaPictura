use super::journal::*;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

struct Temp(PathBuf);

impl Temp {
    fn new() -> Self {
        let id = u64::from_ne_bytes(pictura_codec::os_random());
        let path = std::env::temp_dir().join(format!("pictura-journal-{id:x}"));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn recovery_batches_sync_during_continuous_edits() {
    let start = Instant::now();
    let mut batch = Batch::default();
    batch.push(start, 1);
    for tick in 1..4 {
        let now = start + Duration::from_millis(500 * tick);
        batch.push(now, 1);
        assert!(!batch.due(now));
    }
    let due = start + Duration::from_secs(2);
    batch.push(due, 1);
    assert!(batch.due(due));
    let mut batch = Batch::default();
    batch.push(start, BATCH_BYTES);
    assert!(batch.due(start));
}

#[test]
fn journal_io_failure_keeps_durable_prefix() {
    let dir = Temp::new();
    let path = dir.0.join("1.journal");
    fs::write(&path, []).unwrap();
    let one = frame_record(9, 1, b"first").unwrap();
    let two = frame_record(9, 2, b"second").unwrap();
    append_batch(&path, &one, false).unwrap();
    assert!(append_batch(&path, &two, true).is_err());
    let mut payloads = Vec::new();
    let replay = replay_records(&path, 9, 0, |_, payload| {
        payloads.push(payload.to_vec());
        Ok(())
    })
    .unwrap();
    assert_eq!(replay, (1, false));
    assert_eq!(payloads, [b"first".to_vec()]);
}

#[test]
fn journal_replay_stops_at_first_invalid_record() {
    let dir = Temp::new();
    let path = dir.0.join("2.journal");
    let first = frame_record(9, 1, b"first").unwrap();
    let second = frame_record(9, 2, b"second").unwrap();
    let later = frame_record(9, 3, b"third").unwrap();
    let mut bad_crc = second.clone();
    *bad_crc.last_mut().unwrap() ^= 1;
    let mut oversized = second.clone();
    oversized[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    for tail in [
        bad_crc,
        second[..second.len() - 1].to_vec(),
        later.clone(),
        oversized,
    ] {
        fs::write(&path, [first.as_slice(), &tail, &later].concat()).unwrap();
        let mut payloads = Vec::new();
        let replay = replay_records(&path, 9, 0, |_, payload| {
            payloads.push(payload.to_vec());
            Ok(())
        })
        .unwrap();
        assert_eq!(replay, (1, true));
        assert_eq!(payloads, [b"first".to_vec()]);
    }
}

#[test]
fn interrupted_checkpoint_publication_preserves_journal_suffix() {
    let dir = Temp::new();
    let manifest = dir.0.join("3.json");
    let old_log = dir.0.join("old.journal");
    let new_log = dir.0.join("new.journal");
    let first = frame_record(9, 1, b"first").unwrap();
    let second = frame_record(9, 2, b"second").unwrap();
    fs::write(&old_log, [first.as_slice(), &second].concat()).unwrap();
    fs::write(&new_log, &second).unwrap();
    fs::write(&manifest, b"old").unwrap();
    assert!(publish_manifest(
        &manifest,
        b"new",
        |_| Err(io::Error::other("interrupt")),
        |_| Ok(())
    )
    .is_err());
    assert_eq!(fs::read(&manifest).unwrap(), b"old");
    assert_eq!(
        replay_records(&old_log, 9, 0, |_, _| Ok(())).unwrap(),
        (2, false)
    );
    publish_manifest(&manifest, b"new", |_| Ok(()), |_| Ok(())).unwrap();
    assert_eq!(fs::read(&manifest).unwrap(), b"new");
    assert_eq!(
        replay_records(&new_log, 9, 1, |_, _| Ok(())).unwrap(),
        (2, false)
    );
}
