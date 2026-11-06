//! The memory governor's bridge functions (#1211), declared in the fills
//! bridge: the Memory Usage preference, the governor's state for the control
//! server's `status`, and the test hooks the Qt suites pin a host with.

use cxx_qt_lib::QString;
use pictura_core::memory::{self, MemoryStatus};

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

pub(in crate::cxxqt_object) fn memory_status_json() -> QString {
    QString::from(status_json(&memory::governor().status()).as_str())
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
