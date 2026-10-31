//! Backend selection for the GPU compositor.
//!
//! wgpu's `Instance::new` initializes every backend named in its bitmask, so a
//! multi-backend instance loads every driver — including a crashing one. This
//! module pins exactly one backend per process and resolves the candidate list
//! the launcher probes, one child process per candidate.
//!
//! The pure helpers take the platform and compiled set as inputs so the
//! Windows order can be tested on Linux.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use wgpu::Backends;

/// The driver API a launch may use. Distinct from [`super::Backend`], which
/// reports which compositor produced a buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBackend {
    Dx12,
    Vulkan,
    Cpu,
}

impl GpuBackend {
    /// The wgpu bitmask for this backend. `Cpu` maps to no backend.
    pub fn to_wgpu(self) -> Backends {
        match self {
            Self::Dx12 => Backends::DX12,
            Self::Vulkan => Backends::VULKAN,
            Self::Cpu => Backends::empty(),
        }
    }

    /// Whether this backend was compiled for the target. Compile-time only;
    /// never loads a driver. `Cpu` is never a compiled GPU backend.
    pub fn is_compiled(self) -> bool {
        self != Self::Cpu && wgpu::Instance::enabled_backend_features().contains(self.to_wgpu())
    }

    /// Stable lowercase id used by the cache and the probe argument.
    pub fn name(self) -> &'static str {
        match self {
            Self::Dx12 => "dx12",
            Self::Vulkan => "vulkan",
            Self::Cpu => "cpu",
        }
    }

    /// Bridge code: 0 Vulkan, 1 DX12, 2 Cpu.
    pub fn code(self) -> i32 {
        match self {
            Self::Vulkan => 0,
            Self::Dx12 => 1,
            Self::Cpu => 2,
        }
    }

    /// Inverse of [`Self::code`].
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Vulkan),
            1 => Some(Self::Dx12),
            2 => Some(Self::Cpu),
            _ => None,
        }
    }

    /// Parse a backend name used by `WGPU_BACKEND`.
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "vulkan" | "vk" => Some(Self::Vulkan),
            "dx12" | "d3d12" => Some(Self::Dx12),
            _ => None,
        }
    }
}

/// The target family the default candidate order depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Linux,
    Other,
}

/// The host platform.
pub fn platform() -> Platform {
    if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "linux") {
        Platform::Linux
    } else {
        Platform::Other
    }
}

/// The backends this build compiled for the target.
pub fn compiled_backends() -> Vec<GpuBackend> {
    [GpuBackend::Dx12, GpuBackend::Vulkan]
        .into_iter()
        .filter(|backend| backend.is_compiled())
        .collect()
}

/// Parse `WGPU_BACKEND`. `None` means unset/auto; `Some(list)` is an explicit
/// override (possibly empty for `none` or an unknown name).
fn parse_override(value: Option<&str>) -> Option<Vec<GpuBackend>> {
    let value = value?.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("auto") {
        return None;
    }
    if value.eq_ignore_ascii_case("none")
        || value.eq_ignore_ascii_case("cpu")
        || value.eq_ignore_ascii_case("off")
    {
        return Some(Vec::new());
    }
    let mut backends = Vec::new();
    for name in value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        if let Some(backend) = GpuBackend::from_name(&name.to_ascii_lowercase()) {
            if !backends.contains(&backend) {
                backends.push(backend);
            }
        }
    }
    Some(backends)
}

/// The ordered candidates for a launch, intersected with `compiled`.
///
/// The boolean is true when an explicit override replaced the platform order.
/// An override that names no compiled backend yields an empty list.
pub fn resolve_candidates(
    platform: Platform,
    compiled: &[GpuBackend],
    override_value: Option<&str>,
) -> (Vec<GpuBackend>, bool) {
    let override_backends = parse_override(override_value);
    let order = match &override_backends {
        Some(list) => list.clone(),
        None => match platform {
            Platform::Windows => vec![GpuBackend::Dx12, GpuBackend::Vulkan],
            Platform::Linux => vec![GpuBackend::Vulkan],
            Platform::Other => Vec::new(),
        },
    };
    let candidates = order
        .into_iter()
        .filter(|backend| compiled.contains(backend))
        .collect();
    (candidates, override_backends.is_some())
}

/// The environment-reading wrapper around [`resolve_candidates`].
pub fn env_candidates() -> (Vec<GpuBackend>, bool) {
    let value = std::env::var("WGPU_BACKEND").ok();
    resolve_candidates(platform(), &compiled_backends(), value.as_deref())
}

static SELECTED: OnceLock<GpuBackend> = OnceLock::new();
static SAFE_MODE: AtomicBool = AtomicBool::new(false);

/// The backend this process is pinned to. Resolved once, on first use, from
/// the environment and the compiled backends.
pub fn selected_backend() -> GpuBackend {
    *SELECTED.get_or_init(|| {
        let (candidates, _) = env_candidates();
        candidates.first().copied().unwrap_or(GpuBackend::Cpu)
    })
}

/// Pin the process to one backend. Must run before the first
/// [`selected_backend`]/`gpu_available` call. Returns the current pin when it
/// was already resolved.
pub fn set_backend(backend: GpuBackend) -> Result<(), GpuBackend> {
    match SELECTED.set(backend) {
        Ok(()) => Ok(()),
        Err(_) => Err(selected_backend()),
    }
}

/// Mark the GPU path blocked for this process (chain exhausted or validation
/// failed), so the status label says safe mode rather than "no GPU".
pub fn set_safe_mode(active: bool) {
    SAFE_MODE.store(active, Ordering::Relaxed);
}

/// The process-wide backend status for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBackendStatus {
    Active(GpuBackend),
    NoAdapter,
    SafeMode,
}

/// The current backend status. Cheap after the first probe.
pub fn gpu_backend_status() -> GpuBackendStatus {
    if super::gpu_available() {
        return GpuBackendStatus::Active(selected_backend());
    }
    if super::device_lost() || SAFE_MODE.load(Ordering::Relaxed) {
        GpuBackendStatus::SafeMode
    } else {
        GpuBackendStatus::NoAdapter
    }
}

/// The status bar / About label. Precedence: no adapter, user disabled, safe
/// mode, active backend.
pub fn backend_label(gpu_compute: bool, status: GpuBackendStatus) -> String {
    match status {
        GpuBackendStatus::NoAdapter => "CPU (no GPU)".to_string(),
        _ if !gpu_compute => "CPU".to_string(),
        GpuBackendStatus::SafeMode => "CPU (safe mode)".to_string(),
        GpuBackendStatus::Active(GpuBackend::Dx12) => "GPU (DX12)".to_string(),
        GpuBackendStatus::Active(GpuBackend::Vulkan) => "GPU (Vulkan)".to_string(),
        GpuBackendStatus::Active(_) => "GPU".to_string(),
    }
}

/// The result of probing one backend in a fresh process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// The device was created and reproduced the CPU oracle.
    Ok,
    /// The backend is uncompiled, or no adapter/device exists.
    NoAdapter,
    /// A device exists but failed the CPU-oracle validation.
    Invalid,
}

/// Probe one backend in this process and distinguish "no adapter" from
/// "adapter present but validation failed". Intended for the `--gpu-probe`
/// child, whose `OnceLock`s are fresh.
pub fn probe_backend_outcome(backend: GpuBackend) -> ProbeOutcome {
    if backend == GpuBackend::Cpu || !backend.is_compiled() {
        return ProbeOutcome::NoAdapter;
    }
    match set_backend(backend) {
        Ok(()) => {}
        Err(current) if current == backend => {}
        Err(_) => return ProbeOutcome::NoAdapter,
    }
    if !super::device_available() {
        return ProbeOutcome::NoAdapter;
    }
    if super::gpu_composite_verified() {
        ProbeOutcome::Ok
    } else {
        ProbeOutcome::Invalid
    }
}

/// Probe one backend in this process: pin it, create the device, and run the
/// CPU-oracle validation. Returns false for an uncompiled backend, a missing
/// adapter, or a validation failure. Intended for the `--gpu-probe` child.
pub fn probe_backend(backend: GpuBackend) -> bool {
    probe_backend_outcome(backend) == ProbeOutcome::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win() -> Platform {
        Platform::Windows
    }

    fn linux() -> Platform {
        Platform::Linux
    }

    #[test]
    fn windows_default_candidates_are_dx12_then_vulkan() {
        let compiled = [GpuBackend::Dx12, GpuBackend::Vulkan];
        let (candidates, overridden) = resolve_candidates(win(), &compiled, None);
        assert_eq!(candidates, vec![GpuBackend::Dx12, GpuBackend::Vulkan]);
        assert!(!overridden);
    }

    #[test]
    fn linux_default_candidates_are_vulkan_only() {
        let compiled = [GpuBackend::Vulkan];
        let (candidates, overridden) = resolve_candidates(linux(), &compiled, None);
        assert_eq!(candidates, vec![GpuBackend::Vulkan]);
        assert!(!overridden);
    }

    #[test]
    fn override_selects_exactly_one_backend() {
        let compiled = [GpuBackend::Dx12, GpuBackend::Vulkan];
        let (candidates, overridden) = resolve_candidates(win(), &compiled, Some("vulkan"));
        assert_eq!(candidates, vec![GpuBackend::Vulkan]);
        assert!(overridden);

        let (candidates, overridden) = resolve_candidates(win(), &compiled, Some("dx12"));
        assert_eq!(candidates, vec![GpuBackend::Dx12]);
        assert!(overridden);
    }

    #[test]
    fn none_override_selects_no_backend() {
        let compiled = [GpuBackend::Dx12, GpuBackend::Vulkan];
        let (candidates, overridden) = resolve_candidates(win(), &compiled, Some("none"));
        assert!(candidates.is_empty());
        assert!(overridden);
    }

    #[test]
    fn unsupported_override_selects_no_backend() {
        let compiled = [GpuBackend::Vulkan];
        let (candidates, overridden) = resolve_candidates(linux(), &compiled, Some("dx12"));
        assert!(candidates.is_empty());
        assert!(overridden);

        let (candidates, _) = resolve_candidates(linux(), &compiled, Some("metal"));
        assert!(candidates.is_empty());
    }

    #[test]
    fn candidates_intersect_compiled_backends() {
        let (candidates, _) = resolve_candidates(win(), &[GpuBackend::Vulkan], None);
        assert_eq!(candidates, vec![GpuBackend::Vulkan]);

        let (candidates, _) = resolve_candidates(linux(), &[GpuBackend::Dx12], None);
        assert!(candidates.is_empty());
    }

    #[test]
    fn single_backend_pin_is_one_wgpu_bit() {
        assert_eq!(GpuBackend::Dx12.to_wgpu(), Backends::DX12);
        assert_eq!(GpuBackend::Vulkan.to_wgpu(), Backends::VULKAN);
        assert_eq!(GpuBackend::Cpu.to_wgpu(), Backends::empty());
        assert!(!GpuBackend::Dx12.to_wgpu().contains(Backends::VULKAN));
        assert!(!GpuBackend::Vulkan.to_wgpu().contains(Backends::DX12));
    }

    #[test]
    fn backend_label_names_all_five_states() {
        use GpuBackendStatus::{Active, NoAdapter, SafeMode};
        assert_eq!(backend_label(true, Active(GpuBackend::Dx12)), "GPU (DX12)");
        assert_eq!(
            backend_label(true, Active(GpuBackend::Vulkan)),
            "GPU (Vulkan)"
        );
        assert_eq!(backend_label(false, Active(GpuBackend::Vulkan)), "CPU");
        assert_eq!(backend_label(true, NoAdapter), "CPU (no GPU)");
        assert_eq!(backend_label(true, SafeMode), "CPU (safe mode)");
    }

    #[test]
    fn probe_backend_rejects_an_uncompiled_backend() {
        let uncompiled = [GpuBackend::Dx12, GpuBackend::Vulkan]
            .into_iter()
            .find(|backend| !backend.is_compiled());
        if let Some(backend) = uncompiled {
            assert_eq!(probe_backend_outcome(backend), ProbeOutcome::NoAdapter);
            assert!(!probe_backend(backend));
        }
    }

    #[test]
    fn cpu_probe_is_no_adapter() {
        assert_eq!(
            probe_backend_outcome(GpuBackend::Cpu),
            ProbeOutcome::NoAdapter
        );
        assert!(!probe_backend(GpuBackend::Cpu));
    }

    #[test]
    #[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
    fn vulkan_probe_is_clean_on_a_software_device() {
        assert!(probe_backend(GpuBackend::Vulkan));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn dx12_probe_is_clean_with_or_without_an_adapter() {
        let _ = probe_backend(GpuBackend::Dx12);
    }
}
