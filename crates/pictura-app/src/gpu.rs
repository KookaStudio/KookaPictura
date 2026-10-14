//! M0.5 spike: offscreen wgpu render on Vulkan, read back to CPU.
//!
//! The GPU is an accelerator, not an oracle: this module never panics on a
//! missing adapter and never blocks the CPU fallback. Callers branch on
//! [`GpuRender`].
//!
//! The zero-copy [`create_interop_state`] escape hatch is Vulkan-only. wgpu
//! builds its Vulkan backend only on Windows/Linux/Android/FreeBSD, so on
//! Apple targets it is compiled out and returns `None`.

use std::borrow::Cow;

/// Outcome of an offscreen render. `Unavailable` means no Vulkan adapter /
/// device (or the render failed); the caller keeps its CPU image.
pub enum GpuRender {
    /// No usable Vulkan device.
    Unavailable,
    /// Render succeeded. `distinct` is the number of distinct RGB values seen
    /// in the readback, used to prove the output is not blank.
    Rendered {
        width: u32,
        height: u32,
        rgba: Vec<u8>,
        distinct: usize,
    },
}

/// Fullscreen-triangle gradient. Laid out so the fragment output varies across
/// the whole target regardless of its size (a flat clear would be blank).
const SHADER: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(pos[idx], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let x = fract(frag.x / 97.0);
    let y = fract(frag.y / 89.0);
    return vec4<f32>(x, y, (x + y) * 0.5, 1.0);
}
"#;

/// Render a gradient into an offscreen Rgba8Unorm texture, copy it to a host
/// buffer, and return the packed RGBA bytes.
pub fn render_gradient(width: u32, height: u32) -> GpuRender {
    match pollster::block_on(render_gradient_async(width, height)) {
        Some(result) => result,
        None => GpuRender::Unavailable,
    }
}

async fn create_device() -> Option<(wgpu::Instance, wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    // wgpu::Instance::new panics where no implemented backend is compiled for
    // the target (Apple builds no wgpu Vulkan). Return `None` so the caller
    // keeps its CPU image, as this module's contract promises.
    if !wgpu::Instance::enabled_backend_features().contains(wgpu::Backends::VULKAN) {
        return None;
    }
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })
        .await
        .ok()?;

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("pictura-offscreen"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        })
        .await
        .ok()?;

    Some((instance, adapter, device, queue))
}

async fn render_gradient_async(width: u32, height: u32) -> Option<GpuRender> {
    let (_instance, adapter, device, queue) = create_device().await?;

    let info = adapter.get_info();
    eprintln!(
        "pictura gpu: adapter={:?} backend={:?} device_type={:?}",
        info.name, info.backend, info.device_type
    );

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pictura-offscreen-target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("pictura-gradient"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("pictura-gradient"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    let padded_row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pictura-readback"),
        size: u64::from(padded_row) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pictura-offscreen"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.draw(0..3, 0..1);
    }
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    rx.recv().ok()?.ok()?;

    let mapped = slice.get_mapped_range().ok()?;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for y in 0..height as usize {
        let src = y * padded_row as usize;
        let dst = y * width as usize * 4;
        rgba[dst..dst + width as usize * 4].copy_from_slice(&mapped[src..src + width as usize * 4]);
    }
    drop(mapped);
    buffer.unmap();

    let distinct = distinct_rgb(&rgba);
    Some(GpuRender::Rendered {
        width,
        height,
        rgba,
        distinct,
    })
}

/// Count distinct RGB triplets (alpha ignored).
fn distinct_rgb(rgba: &[u8]) -> usize {
    let mut seen = std::collections::HashSet::new();
    for px in rgba.as_chunks::<4>().0 {
        seen.insert(u32::from(px[0]) << 16 | u32::from(px[1]) << 8 | u32::from(px[2]));
    }
    seen.len()
}

/// Raw Vulkan handles exported from a live wgpu device, for the zero-copy
/// interop attempt.
pub struct VulkanHandles {
    pub instance: u64,
    pub physical_device: u64,
    pub device: u64,
    pub queue_family: u32,
}

/// A wgpu device plus its exported handles. The wgpu objects are kept alive
/// here because the raw handles are only valid while the device lives.
pub struct InteropState {
    pub handles: VulkanHandles,
    pub image: u64,
    pub width: u32,
    pub height: u32,
    #[allow(dead_code)]
    keepalive: (
        wgpu::Instance,
        wgpu::Adapter,
        wgpu::Device,
        wgpu::Queue,
        wgpu::Texture,
    ),
}

/// Create a Vulkan wgpu device and an offscreen texture, exporting the raw
/// VkDevice/VkImage handles through the (unsafe) wgpu-hal escape hatch.
///
/// Vulkan-only: wgpu has no Vulkan backend on Apple targets, so this is
/// compiled out there and the stub returns `None`.
#[cfg(not(target_vendor = "apple"))]
pub fn create_interop_state() -> Option<InteropState> {
    pollster::block_on(async {
        let (width, height) = (512u32, 512u32);
        let (instance, adapter, device, queue) = create_device().await?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pictura-interop-target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        // SAFETY: Vulkan was forced in `create_device`; the wgpu and wgpu-hal
        // types are the same objects. `as_hal` is unsafe because the caller
        // must uphold the lifetime/exclusivity contract of the raw handles,
        // which `InteropState` does by owning the device and texture.
        let (handles, image) = unsafe {
            use ash::vk::Handle;
            let hal_instance = instance.as_hal::<wgpu::hal::api::Vulkan>()?;
            let hal_device = device.as_hal::<wgpu::hal::api::Vulkan>()?;
            let hal_texture = texture.as_hal::<wgpu::hal::api::Vulkan>()?;
            (
                VulkanHandles {
                    instance: hal_instance
                        .shared_instance()
                        .raw_instance()
                        .handle()
                        .as_raw(),
                    physical_device: hal_device.raw_physical_device().as_raw(),
                    device: hal_device.raw_device().handle().as_raw(),
                    queue_family: hal_device.queue_family_index(),
                },
                hal_texture.raw_handle().as_raw(),
            )
        };
        Some(InteropState {
            handles,
            image,
            width,
            height,
            keepalive: (instance, adapter, device, queue, texture),
        })
    })
}

/// Apple targets build wgpu without the Vulkan backend, so there is no
/// interop state to export; the probe reports unavailable and callers keep the
/// CPU path.
#[cfg(target_vendor = "apple")]
pub fn create_interop_state() -> Option<InteropState> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Offscreen render must produce a non-blank image when a Vulkan adapter is
    /// present. Without one the test passes: the CPU path is the fallback.
    #[test]
    fn offscreen_gradient_is_non_blank() {
        match render_gradient(256, 256) {
            GpuRender::Unavailable => eprintln!("no Vulkan adapter; skipping GPU assertion"),
            GpuRender::Rendered {
                distinct,
                width,
                height,
                rgba,
            } => {
                assert_eq!((width, height), (256, 256));
                assert_eq!(rgba.len(), 256 * 256 * 4);
                assert!(
                    distinct >= 2,
                    "rendered output was blank: {distinct} values"
                );
            }
        }
    }
}
