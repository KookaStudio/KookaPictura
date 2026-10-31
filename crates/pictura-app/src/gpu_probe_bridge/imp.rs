use std::process::{Child, Command};
use std::time::{Duration, Instant};

use pictura_render::GpuBackend;

const TIMEOUT: Duration = Duration::from_secs(10);

pub(super) fn gpu_probe(kind: i32, child_command: &str) -> i32 {
    let Some(backend) = GpuBackend::from_code(kind) else {
        return 1;
    };
    if backend == GpuBackend::Cpu || !backend.is_compiled() {
        return 1;
    }
    let mut command = Command::new(child_command);
    command.arg("--gpu-probe").arg(backend.name());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: the probe is a GUI-subsystem binary, but a console
        // build must not flash a window for the child.
        command.creation_flags(0x0800_0000);
    }
    match command.spawn() {
        Ok(mut child) => wait_with_timeout(&mut child, TIMEOUT),
        Err(_) => 1,
    }
}

pub(super) fn gpu_probe_child(kind: i32) -> i32 {
    let Some(backend) = GpuBackend::from_code(kind) else {
        return 1;
    };
    match pictura_render::probe_backend_outcome(backend) {
        pictura_render::ProbeOutcome::Ok => 0,
        pictura_render::ProbeOutcome::NoAdapter => 1,
        pictura_render::ProbeOutcome::Invalid => 2,
    }
}

pub(super) fn gpu_set_backend(kind: i32) -> bool {
    let Some(backend) = GpuBackend::from_code(kind) else {
        return false;
    };
    match pictura_render::set_backend(backend) {
        Ok(()) => true,
        Err(current) => current == backend,
    }
}

pub(super) fn gpu_set_safe_mode(active: bool) {
    pictura_render::set_safe_mode(active);
}

pub(super) fn gpu_candidates() -> Vec<i32> {
    pictura_render::env_candidates()
        .0
        .into_iter()
        .map(GpuBackend::code)
        .collect()
}

pub(super) fn gpu_active_backend_label(gpu_compute: bool) -> String {
    pictura_render::backend_label(gpu_compute, pictura_render::gpu_backend_status())
}

/// 0 = ok, 1 = no adapter/clean failure, 2 = validation failure, crash, or
/// timeout.
fn wait_with_timeout(child: &mut Child, timeout: Duration) -> i32 {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return match status.code() {
                    Some(0) => 0,
                    Some(1) => 1,
                    _ => 2,
                };
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            _ => {
                // A driver stuck in kernel mode may not honor TerminateProcess
                // promptly; reap briefly, then return without an unbounded wait.
                let _ = child.kill();
                let reap_deadline = Instant::now() + Duration::from_millis(500);
                while Instant::now() < reap_deadline {
                    if matches!(child.try_wait(), Ok(Some(_))) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                return 2;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(script: &str) -> Child {
        #[cfg(windows)]
        {
            Command::new("cmd").args(["/C", script]).spawn().unwrap()
        }
        #[cfg(not(windows))]
        {
            Command::new("sh").args(["-c", script]).spawn().unwrap()
        }
    }

    fn hung_script() -> &'static str {
        #[cfg(windows)]
        {
            // `sleep` does not exist in cmd; ping is the portable wait.
            "ping -n 60 127.0.0.1 >NUL"
        }
        #[cfg(not(windows))]
        {
            "sleep 60"
        }
    }

    #[test]
    fn wait_with_timeout_maps_success() {
        let mut child = shell("exit 0");
        assert_eq!(wait_with_timeout(&mut child, TIMEOUT), 0);
    }

    #[test]
    fn wait_with_timeout_maps_failure() {
        let mut child = shell("exit 1");
        assert_eq!(wait_with_timeout(&mut child, TIMEOUT), 1);
    }

    #[test]
    fn wait_with_timeout_kills_a_hung_child() {
        let mut child = shell(hung_script());
        let started = Instant::now();
        assert_eq!(wait_with_timeout(&mut child, Duration::from_millis(100)), 2);
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(child.try_wait().unwrap().is_some());
    }

    #[test]
    fn unknown_backend_code_is_a_clean_failure() {
        assert_eq!(gpu_probe_child(99), 1);
        assert!(!gpu_set_backend(99));
    }
}
