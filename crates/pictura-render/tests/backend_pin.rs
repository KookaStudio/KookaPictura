use pictura_render::{gpu_available, selected_backend, set_backend, GpuBackend};

#[test]
fn late_pin_is_refused_after_resolution() {
    let first = selected_backend();
    let _ = gpu_available();
    let other = if first == GpuBackend::Vulkan {
        GpuBackend::Cpu
    } else {
        GpuBackend::Vulkan
    };
    assert_eq!(set_backend(other), Err(first));
    assert_eq!(selected_backend(), first);
}
