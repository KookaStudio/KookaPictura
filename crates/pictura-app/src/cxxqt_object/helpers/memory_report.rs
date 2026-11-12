//! The memory governor's bridge functions (#1211), declared in the fills
//! bridge: the Memory Usage preference, the governor's state for the control
//! server's `status`, the scratch disk and eviction readouts and volumes
//! (#1217), and the test hooks the Qt suites pin a host with.

use cxx_qt_lib::QString;
use pictura_core::memory::eviction::{self, TileCounts};
use pictura_core::memory::{self, MemoryStatus};
use pictura_core::scratch::{self, Efficiency, Fault, Sizes, VolumeInfo};

pub(in crate::cxxqt_object) fn memory_usage_percent() -> i32 {
    i32::from(memory::governor().usage_percent())
}

pub(in crate::cxxqt_object) fn set_memory_usage_percent(percent: i32) {
    memory::governor().set_usage_percent(percent.clamp(5, 100) as u8);
}

fn status_json(status: &MemoryStatus) -> String {
    let bytes = |value: Option<u64>| value.map_or_else(|| "null".to_string(), |v| v.to_string());
    format!(
        r#"{{"ceiling":{},"used":{},"reserved":{},"room":{},"pressure":"{}","usage_percent":{}}}"#,
        bytes(status.ceiling),
        bytes(status.used),
        status.reserved,
        bytes(status.room),
        status.pressure.name(),
        status.usage_percent
    )
}

/// JSON string literal of `s`.
fn quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The scratch and eviction fields of `status.memory` (#1217): `scratch`
/// (volumes, Scratch Sizes), `efficiency` and `tiles`.
fn scratch_json(
    volumes: &[VolumeInfo],
    sizes: Sizes,
    efficiency: Efficiency,
    tiles: TileCounts,
) -> String {
    let volumes: Vec<String> = volumes
        .iter()
        .map(|v| {
            format!(
                r#"{{"path":{},"file":{},"used":{},"available":{},"full":{},"tmpfs":{}}}"#,
                quoted(&v.path.to_string_lossy()),
                v.file_len,
                v.used,
                v.available,
                v.full,
                v.tmpfs
            )
        })
        .collect();
    format!(
        r#""scratch":{{"volumes":[{}],"used":{},"available":{}}},"efficiency":{{"compute_ms":{},"io_ms":{},"percent":{:.1}}},"tiles":{{"packed":{},"packed_bytes":{},"spilled":{},"unreadable":{}}}"#,
        volumes.join(","),
        sizes.used,
        sizes.available,
        efficiency.compute_ns / 1_000_000,
        efficiency.io_ns / 1_000_000,
        efficiency.percent(),
        tiles.packed,
        tiles.packed_bytes,
        tiles.spilled,
        tiles.unreadable
    )
}

pub(in crate::cxxqt_object) fn memory_status_json() -> QString {
    let mut json = status_json(&memory::governor().status());
    json.pop();
    json.push(',');
    json.push_str(&scratch_json(
        &scratch::volumes(),
        scratch::sizes(),
        scratch::efficiency(),
        eviction::counts(),
    ));
    json.push('}');
    QString::from(json.as_str())
}

/// Turn eviction on and start its thread; the frame calls this at start-up.
pub(in crate::cxxqt_object) fn start_memory_eviction() {
    eviction::start();
}

/// The volumes the user configured, one path a line; empty while the default
/// is in use, so the session keeps following `$XDG_CACHE_HOME`.
static CONFIGURED: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

pub(in crate::cxxqt_object) fn scratch_volumes() -> QString {
    QString::from(
        CONFIGURED
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_str(),
    )
}

/// Use the scratch volumes in `paths` (one a line, at most four); an empty
/// list restores the default.
pub(in crate::cxxqt_object) fn set_scratch_volumes(paths: &QString) {
    let dirs: Vec<std::path::PathBuf> = paths
        .to_string()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(std::path::PathBuf::from)
        .collect();
    *CONFIGURED.lock().unwrap_or_else(|e| e.into_inner()) = dirs
        .iter()
        .map(|d| d.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n");
    scratch::set_volumes(&dirs);
}

/// Test hook: arm a scratch fault (`0` full, `1` read error, `2` corrupt,
/// `3` removed volume, at `volume`); any other kind clears them all.
pub(in crate::cxxqt_object) fn inject_scratch_fault_for_test(kind: i32, volume: i32) {
    let volume = volume.max(0) as usize;
    match kind {
        0 => scratch::inject(Fault::Full { volume }),
        1 => scratch::inject(Fault::ReadError),
        2 => scratch::inject(Fault::Corrupt),
        3 => scratch::inject(Fault::RemoveVolume { volume }),
        _ => scratch::clear_faults(),
    }
}

pub(in crate::cxxqt_object) fn open_refused_for_memory() -> bool {
    super::super::open_jobs::open_refused_for_memory()
}

pub(in crate::cxxqt_object) fn set_memory_cgroup_for_test(max: i64, current: i64) {
    super::memory_gate::set_cgroup_for_test(max, current);
}

pub(in crate::cxxqt_object) fn fail_next_plane_for_test() {
    memory::fail_next_plane_for_test();
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::memory::Pressure;

    #[test]
    fn the_status_json_carries_scratch_readouts() {
        let volumes = [VolumeInfo {
            path: "/var/scr\"atch".into(),
            file_len: 8,
            used: 4,
            available: 100,
            full: false,
            tmpfs: true,
        }];
        let json = scratch_json(
            &volumes,
            Sizes {
                used: 4,
                available: 100,
            },
            Efficiency {
                compute_ns: 3_000_000,
                io_ns: 1_000_000,
            },
            TileCounts {
                packed: 2,
                packed_bytes: 10,
                spilled: 1,
                unreadable: 0,
            },
        );
        assert_eq!(
            json,
            r#""scratch":{"volumes":[{"path":"/var/scr\"atch","file":8,"used":4,"available":100,"full":false,"tmpfs":true}],"used":4,"available":100},"efficiency":{"compute_ms":3,"io_ms":1,"percent":75.0},"tiles":{"packed":2,"packed_bytes":10,"spilled":1,"unreadable":0}"#
        );
        let full = memory_status_json().to_string();
        for key in [
            "\"scratch\"",
            "\"volumes\"",
            "\"efficiency\"",
            "\"percent\"",
            "\"tiles\"",
        ] {
            assert!(full.contains(key), "{key} in {full}");
        }
        assert!(full.starts_with("{\"ceiling\"") && full.ends_with('}'));
    }

    #[test]
    fn the_status_json_writes_null_for_unknown_values() {
        let status = MemoryStatus {
            ceiling: Some(3),
            used: None,
            reserved: 2,
            room: Some(1),
            pressure: Pressure::Elevated,
            usage_percent: 60,
        };
        assert_eq!(
            status_json(&status),
            r#"{"ceiling":3,"used":null,"reserved":2,"room":1,"pressure":"elevated","usage_percent":60}"#
        );
    }
}
