use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use pictura_core::{Document, Layer, PsdRect};

#[derive(Clone, Debug)]
pub(crate) enum Event {
    Resolve(bool),
    Lookup(Option<PsdRect>),
    BuildStart(Option<PsdRect>),
    BuildEnd,
    Ready(usize),
    Stage(&'static str),
    Band,
}

#[derive(Clone, Debug)]
pub(super) struct Record {
    pub layer: usize,
    pub event: Event,
    pub time: Duration,
    pub worker: Option<usize>,
}

pub(super) fn layer_id(layer: &Layer) -> usize {
    layer as *const Layer as usize
}

fn registry() -> &'static Mutex<HashMap<usize, Arc<Probe>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, Arc<Probe>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) struct Probe {
    began: Instant,
    records: Mutex<Vec<Record>>,
    entered: Condvar,
    rendezvous: Vec<usize>,
}

pub(super) struct Guard {
    doc: usize,
    pub probe: Arc<Probe>,
}

impl Guard {
    pub fn new(doc: &Document, rendezvous: Vec<usize>) -> Self {
        let key = doc as *const Document as usize;
        let probe = Arc::new(Probe {
            began: Instant::now(),
            records: Mutex::new(Vec::new()),
            entered: Condvar::new(),
            rendezvous,
        });
        assert!(registry()
            .lock()
            .unwrap()
            .insert(key, Arc::clone(&probe))
            .is_none());
        Self { doc: key, probe }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        registry().lock().unwrap().remove(&self.doc);
    }
}

impl Probe {
    pub fn snapshot(&self) -> Vec<Record> {
        self.records.lock().unwrap().clone()
    }

    fn record(&self, layer: usize, event: Event) {
        let wait = matches!(event, Event::BuildStart(_)) && self.rendezvous.contains(&layer);
        let mut records = self.records.lock().unwrap();
        records.push(Record {
            layer,
            event,
            time: self.began.elapsed(),
            worker: rayon::current_thread_index(),
        });
        self.entered.notify_all();
        if wait {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !self.rendezvous.iter().all(|key| {
                records.iter().any(|record| {
                    record.layer == *key && matches!(record.event, Event::BuildStart(_))
                })
            }) {
                let remaining = deadline.saturating_duration_since(Instant::now());
                // Drop the lock before panicking so an accidentally serial traversal
                // fails the test without poisoning the observer or wedging its peer.
                if remaining.is_zero() {
                    drop(records);
                    panic!("independent effect builds did not enter concurrently");
                }
                records = self.entered.wait_timeout(records, remaining).unwrap().0;
            }
        }
    }
}

pub(crate) fn observe(doc: &Document, layer: Option<&Layer>, event: Event) {
    let probe = registry()
        .lock()
        .unwrap()
        .get(&(doc as *const Document as usize))
        .cloned();
    if let Some(probe) = probe {
        probe.record(layer.map(layer_id).unwrap_or(0), event);
    }
}

pub(super) fn count(records: &[Record], layer: &Layer, matches: impl Fn(&Event) -> bool) -> usize {
    records
        .iter()
        .filter(|r| r.layer == layer_id(layer) && matches(&r.event))
        .count()
}

pub(super) fn assert_replay_only(records: &[Record]) {
    let band = records
        .iter()
        .position(|r| matches!(r.event, Event::Band))
        .expect("a band rendered");
    assert!(records[..band]
        .iter()
        .any(|r| matches!(r.event, Event::BuildEnd)));
    assert!(records[band..]
        .iter()
        .all(|r| matches!(r.event, Event::Band)));
}
