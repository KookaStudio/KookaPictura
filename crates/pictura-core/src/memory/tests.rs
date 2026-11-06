use super::system::{cgroup_limit, cgroup_path, mem_available, meminfo_bytes, psi_some_avg10};
use super::*;
use std::path::{Path, PathBuf};

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

fn host(total: u64) -> HostReading {
    HostReading {
        total: Some(total),
        ..HostReading::default()
    }
}

#[test]
fn a_cgroup_limit_caps_the_ceiling() {
    let mut reading = host(32 * GIB);
    reading.cgroup_max = Some(4 * GIB);
    assert_eq!(ceiling_of(&reading, 60), Some(4 * GIB - 256 * MIB));
    let text = "max\n";
    reading.cgroup_max = cgroup_limit(text);
    assert_eq!(reading.cgroup_max, None, "`max` is no limit");
    let usage = 32 * GIB * 60 / 100;
    assert_eq!(ceiling_of(&reading, 60), Some(usage - usage / 20));
}

/// A fake host under a fresh temp directory, removed on drop.
struct FakeRoot(PathBuf);

impl FakeRoot {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "pictura-memory-{}-{tag}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp dir");
        Self(dir)
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("dirs");
        std::fs::write(path, text).expect("a file");
    }

    fn paths(&self) -> HostPaths {
        HostPaths {
            proc_root: self.0.join("proc"),
            cgroup_root: self.0.join("cgroup"),
        }
    }

    fn root(&self) -> &Path {
        &self.0
    }
}

impl Drop for FakeRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn nested_host(root: &FakeRoot) {
    root.write(
        "proc/meminfo",
        "MemTotal:       33554432 kB\nMemFree: 1 kB\nMemAvailable:   16777216 kB\n",
    );
    root.write("proc/self/status", "Name: pictura\nVmRSS:\t  524288 kB\n");
    root.write(
        "proc/self/cgroup",
        "0::/user.slice/app.slice/pictura.scope\n",
    );
    root.write(
        "proc/pressure/memory",
        "some avg10=7.25 avg60=1.00 avg300=0.10 total=1\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=0\n",
    );
    root.write("cgroup/user.slice/memory.max", "2147483648\n");
    root.write("cgroup/user.slice/app.slice/memory.max", "max\n");
    root.write(
        "cgroup/user.slice/app.slice/pictura.scope/memory.max",
        "max\n",
    );
    root.write(
        "cgroup/user.slice/app.slice/pictura.scope/memory.high",
        "3221225472\n",
    );
    root.write(
        "cgroup/user.slice/app.slice/pictura.scope/memory.current",
        "1073741824\n",
    );
}

#[test]
fn an_ancestor_limit_and_memory_high_count() {
    let root = FakeRoot::new("nested");
    nested_host(&root);
    let reading = system::read(&root.paths());
    assert_eq!(reading.cgroup_max, Some(2 * GIB), "the ancestor's limit");
    assert_eq!(reading.cgroup_high, Some(3 * GIB));
    assert_eq!(ceiling_of(&reading, 100), Some(2 * GIB - 256 * MIB));
}

#[test]
fn headroom_is_the_larger_of_five_percent_or_256_mib() {
    assert_eq!(less_headroom(GIB), 768 * MIB);
    assert_eq!(less_headroom(10 * GIB), 10 * GIB - 512 * MIB);
    assert_eq!(less_headroom(200 * MIB), 0);
}

#[test]
fn an_unreadable_host_refuses_nothing() {
    let reading = HostReading::default();
    assert_eq!(ceiling_of(&reading, 60), None);
    assert_eq!(room_of(&reading, 60, 0), None);
    let governor = Governor::new(HostPaths::default());
    set_host_for_test(Some(reading));
    assert!(governor.admit(1 << 40, "a test"));
    set_host_for_test(None);
}

#[test]
fn use_reservations_and_available_memory_shrink_the_room() {
    // A 3 GiB ceiling: a cgroup limit of 3 GiB + 256 MiB.
    let mut reading = host(64 * GIB);
    reading.cgroup_max = Some(3 * GIB + 256 * MIB);
    reading.cgroup_current = Some(GIB);
    reading.rss = Some(10 * GIB);
    assert_eq!(ceiling_of(&reading, 100), Some(3 * GIB));
    assert_eq!(
        used_of(&reading),
        Some(GIB),
        "the cgroup's use, not the RSS"
    );
    assert_eq!(room_of(&reading, 100, 512 * MIB), Some(3 * GIB / 2));
    reading.available = Some(GIB);
    assert_eq!(room_of(&reading, 100, 512 * MIB), Some(256 * MIB));
}

#[test]
fn psi_sets_the_pressure_level() {
    let level = |avg10: &str| {
        let text = format!("some avg10={avg10} avg60=0.00 avg300=0.00 total=0\n");
        pressure_of(&HostReading {
            psi_some_avg10: psi_some_avg10(&text),
            ..host(GIB)
        })
    };
    assert_eq!(level("0.50"), Pressure::Normal);
    assert_eq!(level("7.25"), Pressure::Elevated);
    assert_eq!(level("31.00"), Pressure::Critical);
}

#[test]
fn the_free_share_sets_the_level_without_psi() {
    let level = |available| {
        pressure_of(&HostReading {
            available: Some(available),
            ..host(100 * GIB)
        })
    };
    assert_eq!(level(50 * GIB), Pressure::Normal);
    assert_eq!(level(15 * GIB), Pressure::Elevated);
    assert_eq!(level(5 * GIB), Pressure::Critical);
}

#[test]
fn a_reservation_is_released_on_drop() {
    let governor = Governor::new(HostPaths::default());
    governor.set_usage_percent(100);
    let mut reading = host(64 * GIB);
    reading.cgroup_max = Some(3 * GIB / 2 + 256 * MIB);
    reading.cgroup_current = Some(0);
    set_host_for_test(Some(reading));
    clear_refusal();
    let first = governor.reserve(GIB, "first").expect("room for the first");
    assert_eq!(governor.reserved(), GIB);
    let refused = governor.reserve(GIB, "second").err();
    let expected = MemoryError::Insufficient {
        purpose: "second",
        bytes: GIB,
    };
    assert_eq!(refused, Some(expected));
    assert_eq!(last_refusal(), Some(expected));
    drop(first);
    assert_eq!(governor.reserved(), 0);
    assert!(governor.reserve(GIB, "second").is_ok());
    set_host_for_test(None);
    clear_refusal();
}

#[test]
fn a_host_is_read_from_injected_roots() {
    let root = FakeRoot::new("roots");
    nested_host(&root);
    let reading = system::read(&root.paths());
    assert_eq!(reading.total, Some(32 * GIB));
    assert_eq!(reading.available, Some(16 * GIB));
    assert_eq!(reading.rss, Some(512 * MIB));
    assert_eq!(reading.cgroup_current, Some(GIB));
    assert_eq!(reading.psi_some_avg10, Some(7.25));
    std::fs::remove_file(root.root().join("proc/pressure/memory")).expect("removed");
    let without = system::read(&root.paths());
    assert_eq!(without.psi_some_avg10, None);
    assert_eq!(
        HostReading {
            psi_some_avg10: Some(7.25),
            ..without
        },
        reading,
        "only PSI is missing"
    );
}

#[test]
fn the_reading_is_cached_for_250_ms() {
    let root = FakeRoot::new("cache");
    nested_host(&root);
    let governor = Governor::new(root.paths());
    let first = governor.reading();
    root.write("proc/self/status", "VmRSS:\t 1 kB\n");
    assert_eq!(governor.reading(), first, "within 250 ms");
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert_eq!(governor.reading().rss, Some(1024), "re-read after 250 ms");
}

#[test]
fn a_plane_is_built_or_refused() {
    let big = 32 * MIB as usize;
    let roomy = HostReading {
        available: Some(64 * GIB),
        ..HostReading::default()
    };
    set_host_for_test(Some(roomy));
    assert_eq!(try_plane(1024, 7, "small").expect("built"), vec![7u8; 1024]);
    let plane = try_plane(big, 9, "big").expect("built");
    assert_eq!((plane.len(), plane[big - 1]), (big, 9));
    drop(plane);
    set_host_for_test(Some(HostReading {
        available: Some(0),
        ..HostReading::default()
    }));
    clear_refusal();
    let refused = MemoryError::Insufficient {
        purpose: "big",
        bytes: big as u64,
    };
    assert_eq!(try_plane(big, 0, "big"), Err(refused));
    assert_eq!(last_refusal(), Some(refused));
    assert!(
        try_plane(1024, 0, "small").is_ok(),
        "small buffers are not charged"
    );
    fail_next_plane_for_test();
    assert_eq!(
        try_plane(1024, 0, "failpoint"),
        Err(MemoryError::Insufficient {
            purpose: "failpoint",
            bytes: 1024
        })
    );
    assert!(
        try_plane(1024, 0, "after").is_ok(),
        "the failpoint fires once"
    );
    set_host_for_test(None);
    clear_refusal();
}

#[test]
fn meminfo_available_is_parsed_in_bytes() {
    let text = "MemTotal:       32000000 kB\nMemFree: 1 kB\nMemAvailable:   22298680 kB\n";
    assert_eq!(mem_available(text), Some(22_298_680 * 1024));
    assert_eq!(mem_available("MemTotal: 5 kB\n"), None);
    assert_eq!(mem_available("MemAvailable: lots\n"), None);
    assert_eq!(meminfo_bytes(text, "MemTotal:"), Some(32_000_000 * 1024));
    assert_eq!(cgroup_path("1:name=systemd:/x\n0::/a/b\n"), Some("/a/b"));
}

#[test]
fn the_host_reports_available_memory() {
    assert!(available_memory().is_some_and(|bytes| bytes > 0));
    let total = total_memory().expect("the host's total memory");
    assert!(available_memory().is_some_and(|available| available <= total));
}
