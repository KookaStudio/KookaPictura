//! Reading the host's memory (#1211): each value is a pure parser over a file's
//! text plus a read under injectable roots, so a test can stand up a fake
//! `/proc` and cgroup tree. A file that cannot be read is `None`, never an
//! error.

use std::path::{Path, PathBuf};

/// Where the host's memory files live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPaths {
    pub proc_root: PathBuf,
    pub cgroup_root: PathBuf,
}

impl Default for HostPaths {
    fn default() -> Self {
        Self {
            proc_root: PathBuf::from("/proc"),
            cgroup_root: PathBuf::from("/sys/fs/cgroup"),
        }
    }
}

/// One reading of the host, in bytes; `None` where it could not be read.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HostReading {
    pub total: Option<u64>,
    pub available: Option<u64>,
    /// The process's resident set.
    pub rss: Option<u64>,
    /// The smallest `memory.max` of the process's cgroup and its ancestors.
    pub cgroup_max: Option<u64>,
    /// The smallest `memory.high` of the process's cgroup and its ancestors.
    pub cgroup_high: Option<u64>,
    /// The process cgroup's `memory.current`.
    pub cgroup_current: Option<u64>,
    /// `some avg10` of `/proc/pressure/memory`, in percent.
    pub psi_some_avg10: Option<f64>,
}

/// Read the host under `paths`.
pub fn read(paths: &HostPaths) -> HostReading {
    let text = |path: PathBuf| std::fs::read_to_string(path).ok();
    let proc = &paths.proc_root;
    let meminfo = text(proc.join("meminfo"));
    let (mut total, mut available) = (
        meminfo
            .as_deref()
            .and_then(|m| meminfo_bytes(m, "MemTotal:")),
        meminfo.as_deref().and_then(mem_available),
    );
    if meminfo.is_none() {
        total = os::total_memory();
        available = os::available_memory();
    }
    let rss = text(proc.join("self/status")).and_then(|s| meminfo_bytes(&s, "VmRSS:"));
    let leaf = text(proc.join("self/cgroup")).and_then(|s| cgroup_path(&s).map(str::to_string));
    let (cgroup_max, cgroup_high, cgroup_current) = match leaf {
        Some(leaf) => {
            let dirs = cgroup_dirs(&paths.cgroup_root, &leaf);
            let smallest = |file: &str| {
                dirs.iter()
                    .filter_map(|dir| text(dir.join(file)).and_then(|t| cgroup_limit(&t)))
                    .min()
            };
            let current = dirs
                .first()
                .and_then(|dir| text(dir.join("memory.current")))
                .and_then(|t| t.trim().parse().ok());
            (smallest("memory.max"), smallest("memory.high"), current)
        }
        None => (None, None, None),
    };
    let psi_some_avg10 = text(proc.join("pressure/memory")).and_then(|t| psi_some_avg10(&t));
    HostReading {
        total,
        available,
        rss,
        cgroup_max,
        cgroup_high,
        cgroup_current,
        psi_some_avg10,
    }
}

/// The process's cgroup directory and each ancestor below the root, leaf
/// first.
fn cgroup_dirs(root: &Path, leaf: &str) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut path = leaf.trim_matches('/');
    while !path.is_empty() {
        dirs.push(root.join(path));
        path = path.rsplit_once('/').map_or("", |(parent, _)| parent);
    }
    dirs
}

/// The cgroup v2 path of a `/proc/self/cgroup` text (its `0::` line).
pub fn cgroup_path(text: &str) -> Option<&str> {
    text.lines().find_map(|line| line.strip_prefix("0::"))
}

/// A `memory.max` or `memory.high` value in bytes; `max` is no limit.
pub fn cgroup_limit(text: &str) -> Option<u64> {
    text.trim().parse().ok()
}

/// `some avg10` of a `/proc/pressure/memory` text.
pub fn psi_some_avg10(text: &str) -> Option<f64> {
    let line = text.lines().find(|l| l.starts_with("some "))?;
    line.split_whitespace()
        .find_map(|field| field.strip_prefix("avg10="))?
        .parse()
        .ok()
}

/// The `MemAvailable` line of a `/proc/meminfo` text, in bytes.
pub fn mem_available(meminfo: &str) -> Option<u64> {
    meminfo_bytes(meminfo, "MemAvailable:")
}

/// The `key` line (`"MemTotal:"`, `"VmRSS:"`, …) of a `/proc` text in kB, in
/// bytes.
pub fn meminfo_bytes(text: &str, key: &str) -> Option<u64> {
    let line = text.lines().find(|l| l.starts_with(key))?;
    let mut fields = line[key.len()..].split_whitespace();
    let kib: u64 = fields.next()?.parse().ok()?;
    (fields.next() == Some("kB")).then(|| kib.saturating_mul(1024))
}

/// The memory the host can give a new allocation now without swapping, in
/// bytes, or `None` when it cannot be read.
pub fn available_memory() -> Option<u64> {
    read_default().available
}

/// The host's physical memory in bytes, or `None` when it cannot be read.
pub fn total_memory() -> Option<u64> {
    read_default().total
}

fn read_default() -> HostReading {
    #[cfg(target_os = "linux")]
    {
        let meminfo = std::fs::read_to_string("/proc/meminfo").ok();
        HostReading {
            total: meminfo
                .as_deref()
                .and_then(|m| meminfo_bytes(m, "MemTotal:")),
            available: meminfo.as_deref().and_then(mem_available),
            ..HostReading::default()
        }
    }
    #[cfg(not(target_os = "linux"))]
    HostReading {
        total: os::total_memory(),
        available: os::available_memory(),
        ..HostReading::default()
    }
}

#[cfg(target_os = "linux")]
mod os {
    // `/proc/meminfo` is read by the caller; nothing else to ask.
    pub(super) fn available_memory() -> Option<u64> {
        None
    }

    pub(super) fn total_memory() -> Option<u64> {
        None
    }
}

#[cfg(target_os = "macos")]
mod os {
    /// Free, inactive, and purgeable pages from the Mach VM statistics: the
    /// pages macOS can hand out without paging anything else out.
    #[allow(deprecated)] // libc points `mach_host_self` at the mach2 crate.
    pub(super) fn available_memory() -> Option<u64> {
        let mut stats: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
        let mut count = libc::HOST_VM_INFO64_COUNT;
        // SAFETY: `stats` is a zeroed `vm_statistics64` and `count` its size in
        // `integer_t` units, as `host_statistics64` requires.
        let status = unsafe {
            libc::host_statistics64(
                libc::mach_host_self(),
                libc::HOST_VM_INFO64,
                (&mut stats as *mut libc::vm_statistics64).cast(),
                &mut count,
            )
        };
        // SAFETY: `sysconf` has no preconditions.
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if status != libc::KERN_SUCCESS || page <= 0 {
            return None;
        }
        let pages = u64::from(stats.free_count)
            + u64::from(stats.inactive_count)
            + u64::from(stats.purgeable_count);
        Some(pages * page as u64)
    }

    /// `hw.memsize`: the physical memory installed.
    pub(super) fn total_memory() -> Option<u64> {
        let mut bytes: u64 = 0;
        let mut len = std::mem::size_of::<u64>();
        // SAFETY: the name is NUL-terminated and `bytes`/`len` describe a
        // `u64` buffer, which is what `hw.memsize` returns.
        let status = unsafe {
            libc::sysctlbyname(
                c"hw.memsize".as_ptr(),
                (&mut bytes as *mut u64).cast(),
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        (status == 0 && bytes > 0).then_some(bytes)
    }
}

#[cfg(windows)]
mod os {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    fn status() -> Option<MEMORYSTATUSEX> {
        // SAFETY: MEMORYSTATUSEX is plain data; `dwLength` is set as the API
        // requires before the call fills the rest.
        let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        (unsafe { GlobalMemoryStatusEx(&mut status) } != 0).then_some(status)
    }

    /// `ullAvailPhys`: physical memory available without paging.
    pub(super) fn available_memory() -> Option<u64> {
        status().map(|s| s.ullAvailPhys)
    }

    /// `ullTotalPhys`: the physical memory installed.
    pub(super) fn total_memory() -> Option<u64> {
        status().map(|s| s.ullTotalPhys)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod os {
    pub(super) fn available_memory() -> Option<u64> {
        None
    }

    pub(super) fn total_memory() -> Option<u64> {
        None
    }
}
