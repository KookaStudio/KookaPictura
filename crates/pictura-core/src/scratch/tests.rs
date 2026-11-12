use super::*;
use std::io::{BufRead, BufReader};
use std::sync::MutexGuard;
use std::time::Duration;

/// Faults are process-wide: tests that arm them, or that the faults would
/// disturb, run one at a time.
pub(crate) fn serial() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_faults();
    guard
}

/// A fresh directory on disk (not the tmpfs `/tmp`) for one test's volumes.
pub(crate) fn temp_volume(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = default_volume()
        .with_file_name("test-scratch")
        .join(format!("{name}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a test volume");
    dir
}

/// A seeded payload of `len` bytes: half runs, half noise.
fn payload(seed: u64, len: usize) -> Vec<u8> {
    let mut s = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
    (0..len)
        .map(|i| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            if (i / 64) % 2 == 0 {
                (i / 512) as u8
            } else {
                s as u8
            }
        })
        .collect()
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|it| it.flatten().map(|e| e.path()).collect())
        .unwrap_or_default()
}

#[test]
fn scratch_round_trip() {
    let _serial = serial();
    let mut store = Store::new(&[temp_volume("round-trip")]);
    let sizes = [1, 7, 4095, 4096, 4097, 65_536, 300 * 1024];
    let written: Vec<_> = sizes
        .iter()
        .enumerate()
        .map(|(i, &len)| {
            let bytes = payload(i as u64, len);
            (store.write(&bytes).expect("room"), bytes)
        })
        .collect();
    let mut out = Vec::new();
    for (extent, bytes) in &written {
        store.read(extent, &mut out).expect("readable");
        assert_eq!(&out, bytes);
    }
}

#[test]
fn scratch_corrupted_extent_is_unreadable() {
    let _serial = serial();
    let mut store = Store::new(&[temp_volume("corrupt")]);
    let bytes = payload(3, 10_000);
    inject(Fault::Corrupt);
    let bad = store.write(&bytes).expect("room");
    clear_faults();
    let mut out = Vec::new();
    assert_eq!(store.read(&bad, &mut out), Err(ScratchError::Unreadable));
    let good = store.write(&bytes).expect("room");
    inject(Fault::ReadError);
    assert_eq!(store.read(&good, &mut out), Err(ScratchError::Unreadable));
    clear_faults();
    store
        .read(&good, &mut out)
        .expect("readable once the fault clears");
    assert_eq!(out, bytes);
}

#[test]
fn scratch_frees_and_reuses_extents() {
    let _serial = serial();
    let mut store = Store::new(&[temp_volume("reuse")]);
    let lens: Vec<usize> = (0..10).map(|i| 5_000 + i * 3_000).collect();
    let first: Vec<_> = lens
        .iter()
        .map(|&l| store.write(&payload(l as u64, l)).unwrap())
        .collect();
    let total: u64 = first.iter().map(Extent::stored).sum();
    assert_eq!(store.sizes().used, total);
    let file_len = store.file_len(0);
    for e in first {
        store.free(e);
    }
    assert_eq!(store.sizes().used, 0);
    for &l in &lens {
        store.write(&payload(l as u64 + 1, l)).unwrap();
    }
    assert_eq!(store.file_len(0), file_len, "freed extents are reused");
    assert_eq!(store.sizes().used, total);
}

#[test]
fn scratch_full_volume_falls_through() {
    let _serial = serial();
    let (a, b) = (temp_volume("full-a"), temp_volume("full-b"));
    let mut store = Store::new(&[a, b]);
    inject(Fault::Full { volume: 0 });
    let bytes = payload(9, 20_000);
    let extent = store.write(&bytes).expect("the second volume has room");
    assert_eq!(extent.volume, 1);
    assert!(store.volumes()[0].full);
    let mut out = Vec::new();
    store.read(&extent, &mut out).unwrap();
    assert_eq!(out, bytes);
}

#[test]
fn scratch_all_full_is_typed() {
    let _serial = serial();
    let mut store = Store::new(&[temp_volume("all-full-a"), temp_volume("all-full-b")]);
    let bytes = payload(4, 8_000);
    let early = store.write(&bytes).unwrap();
    inject(Fault::Full { volume: 0 });
    inject(Fault::Full { volume: 1 });
    assert_eq!(store.write(&bytes), Err(ScratchError::Full));
    let mut out = Vec::new();
    store
        .read(&early, &mut out)
        .expect("earlier extents stay readable");
    assert_eq!(out, bytes);
}

#[test]
fn scratch_removed_volume_falls_through() {
    let _serial = serial();
    let mut store = Store::new(&[temp_volume("removed-a"), temp_volume("removed-b")]);
    let bytes = payload(5, 6_000);
    let on_first = store.write(&bytes).unwrap();
    assert_eq!(on_first.volume, 0);
    inject(Fault::RemoveVolume { volume: 0 });
    let on_second = store.write(&bytes).unwrap();
    assert_eq!(on_second.volume, 1);
    let mut out = Vec::new();
    assert_eq!(
        store.read(&on_first, &mut out),
        Err(ScratchError::Unreadable)
    );
    store.read(&on_second, &mut out).unwrap();
    assert_eq!(out, bytes);
}

#[test]
fn scratch_default_volume_and_tmpfs_flag() {
    let _serial = serial();
    let old = std::env::var_os("XDG_CACHE_HOME");
    // SAFETY: the serial lock keeps other tests of this module off the
    // environment while it changes.
    unsafe { std::env::set_var("XDG_CACHE_HOME", "/var/cache-test-xdg") };
    let resolved = default_volume();
    match old {
        // SAFETY: as above.
        Some(v) => unsafe { std::env::set_var("XDG_CACHE_HOME", v) },
        None => unsafe { std::env::remove_var("XDG_CACHE_HOME") },
    }
    assert_eq!(
        resolved,
        PathBuf::from("/var/cache-test-xdg/kooka-pictura/scratch")
    );
    if is_tmpfs(Path::new("/dev/shm")) {
        let store = Store::new(&[PathBuf::from("/dev/shm/pictura-scratch-test")]);
        assert!(
            store.volumes()[0].tmpfs,
            "a tmpfs volume carries the warning"
        );
    }
    let disk = Store::new(&[temp_volume("not-tmpfs")]);
    assert!(!disk.volumes()[0].tmpfs);
}

#[test]
fn scratch_sizes_follow_extents() {
    let _serial = serial();
    let mut store = Store::new(&[temp_volume("sizes")]);
    assert_eq!(store.sizes().used, 0);
    let a = store.write(&payload(1, 100)).unwrap();
    let b = store.write(&payload(2, 9_000)).unwrap();
    assert_eq!(store.sizes().used, 4096 + 12_288);
    assert!(store.sizes().available > 0);
    store.free(a);
    store.free(b);
    assert_eq!(store.sizes().used, 0);
}

#[test]
fn scratch_efficiency_accounting() {
    let _serial = serial();
    let before = efficiency();
    {
        let _op = operation();
        std::thread::sleep(Duration::from_millis(20));
        let _io = io_timer();
        std::thread::sleep(Duration::from_millis(10));
    }
    let after = efficiency();
    assert!(after.compute_ns >= before.compute_ns && after.io_ns >= before.io_ns);
    let compute = (after.compute_ns - before.compute_ns) as f64 / 1e6;
    let io = (after.io_ns - before.io_ns) as f64 / 1e6;
    assert!((18.0..40.0).contains(&compute), "compute {compute} ms");
    assert!((9.0..30.0).contains(&io), "io {io} ms");
    let percent = after.percent_since(&before);
    assert!((40.0..80.0).contains(&percent), "efficiency {percent} %");
    assert_eq!(after.percent_since(&after), 100.0);
}

/// The child half of [`scratch_sigkill_leaves_no_file`]: spill to the volume
/// it is given, report, and wait to be killed.
#[test]
#[ignore = "run as a child process by scratch_sigkill_leaves_no_file"]
fn scratch_sigkill_child() {
    let Some(dir) = std::env::var_os("PICTURA_SCRATCH_CHILD") else {
        return;
    };
    let mut store = Store::new(&[PathBuf::from(dir)]);
    for i in 0..8 {
        store.write(&payload(i, 50_000)).expect("room");
    }
    println!("pictura-scratch-ready");
    std::thread::sleep(Duration::from_secs(60));
}

#[test]
fn scratch_sigkill_leaves_no_file() {
    let _serial = serial();
    let dir = temp_volume("sigkill");
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "scratch::tests::scratch_sigkill_child",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env("PICTURA_SCRATCH_CHILD", &dir)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("the test binary runs as a child");
    let ready = BufReader::new(child.stdout.take().unwrap())
        .lines()
        .map_while(Result::ok)
        .any(|line| line.contains("pictura-scratch-ready"));
    assert!(ready, "the child spilled");
    child.kill().expect("SIGKILL");
    child.wait().unwrap();
    assert!(entries(&dir).is_empty(), "left behind: {:?}", entries(&dir));
    let mut store = Store::new(std::slice::from_ref(&dir));
    let bytes = payload(77, 30_000);
    let extent = store.write(&bytes).unwrap();
    let mut out = Vec::new();
    store.read(&extent, &mut out).unwrap();
    assert_eq!(out, bytes);
    assert!(entries(&dir).is_empty());
}

/// R8 of #1217: `pread`/`pwrite` through the store against an `mmap` of a
/// file of the same size, for 64 KiB payloads.
///
/// `cargo test --release -p pictura-core -- scratch_io_benchmark --ignored --nocapture`
#[test]
#[ignore = "benchmark: timing on a real disk"]
#[cfg(target_os = "linux")]
fn scratch_io_benchmark() {
    use std::os::fd::AsRawFd;
    use std::time::Instant;
    let _serial = serial();
    let dir = temp_volume("io-bench");
    let tile = 64 * 1024;
    let count = 4096; // 256 MiB
    let data: Vec<Vec<u8>> = (0..count).map(|i| payload(i as u64, tile)).collect();
    let mib = (tile * count) as f64 / (1024.0 * 1024.0);

    let mut store = Store::new(std::slice::from_ref(&dir));
    let start = Instant::now();
    let extents: Vec<_> = data.iter().map(|d| store.write(d).unwrap()).collect();
    let write = start.elapsed();
    let start = Instant::now();
    let mut out = Vec::new();
    for e in &extents {
        store.read(e, &mut out).unwrap();
    }
    let read = start.elapsed();
    println!(
        "pread/pwrite: write {:6.0} MiB/s  read {:6.0} MiB/s",
        mib / write.as_secs_f64(),
        mib / read.as_secs_f64()
    );

    let path = dir.join("mmap-bench");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    let len = tile * count;
    file.set_len(len as u64).unwrap();
    // SAFETY: a private benchmark mapping of a file this test owns, unmapped
    // below; no other reference to it exists.
    unsafe {
        let map = libc::mmap(
            std::ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            file.as_raw_fd(),
            0,
        );
        assert_ne!(map, libc::MAP_FAILED);
        let bytes = std::slice::from_raw_parts_mut(map.cast::<u8>(), len);
        let start = Instant::now();
        for (i, d) in data.iter().enumerate() {
            bytes[i * tile..(i + 1) * tile].copy_from_slice(d);
        }
        libc::msync(map, len, libc::MS_ASYNC);
        let write = start.elapsed();
        libc::madvise(map, len, libc::MADV_DONTNEED);
        let start = Instant::now();
        let mut sum = 0u64;
        for i in 0..count {
            out.clear();
            out.extend_from_slice(&bytes[i * tile..(i + 1) * tile]);
            sum = sum.wrapping_add(u64::from(crc32fast::hash(&out)));
        }
        let read = start.elapsed();
        libc::munmap(map, len);
        println!(
            "mmap:         write {:6.0} MiB/s  read {:6.0} MiB/s  ({sum})",
            mib / write.as_secs_f64(),
            mib / read.as_secs_f64()
        );
    }
}
