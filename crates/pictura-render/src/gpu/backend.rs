use std::borrow::Cow;
use std::sync::{Mutex, OnceLock};

use pictura_core::{BlendMode, Document, Layer, PixelBuffer};

use crate::decode_adjustment;

use super::assemble::{mask_writer, padded, source_geom, source_writer, Region, RUN};
use super::resident::{mask_key, source_key, Resident, UploadKey};
use super::shader::{PLANAR_SHADER, SHADER, STROKE_SHADER};
use super::{adjustment_params, mode_id, storage_entry, GpuError, NO_ADJ};

/// Bind-group layout and compute pipeline, both size-independent (buffer sizes
/// travel as bindings), so created once per shared device instead of rebuilt on
/// every composite. The params uniform lives on [`Gpu`] instead: it carries
/// per-dispatch state, so sharing one across concurrent composites raced.
struct ComputeResources {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}

impl ComputeResources {
    fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pictura-blend-layout"),
            entries: &[
                storage_entry(0, false),
                storage_entry(1, true),
                storage_entry(2, true),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pictura-blend"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pictura-blend"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pictura-blend"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { pipeline, layout }
    }
}

/// Bind-group layout and compute pipeline for the planar readback pass. Like
/// [`ComputeResources`] it is size-independent (the canvas and planar sizes and
/// the grid stride travel as bindings), so created once per device.
pub(super) struct PlanarResources {
    pub(super) pipeline: wgpu::ComputePipeline,
    pub(super) layout: wgpu::BindGroupLayout,
}

impl PlanarResources {
    fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pictura-planar-layout"),
            entries: &[
                storage_entry(0, true),
                storage_entry(1, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pictura-planar"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(PLANAR_SHADER)),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pictura-planar"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pictura-planar"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("cs_planar"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { pipeline, layout }
    }
}

/// Bind-group layout and compute pipeline for one paint dab. Like the others,
/// size-independent (sizes travel as bindings), so created once per device.
pub(super) struct StrokeResources {
    pub(super) pipeline: wgpu::ComputePipeline,
    pub(super) layout: wgpu::BindGroupLayout,
}

impl StrokeResources {
    fn new(device: &wgpu::Device) -> Self {
        // 0 layer (read-write), 1 coverage (read-write), 2 base (read-only),
        // 3 params. The dab recomposites each changed pixel from the immutable
        // base, exactly as the CPU stroke does, so overlapping dabs cannot
        // compound.
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pictura-stroke-layout"),
            entries: &[
                storage_entry(0, false),
                storage_entry(1, false),
                storage_entry(2, true),
                storage_entry(3, true),
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pictura-stroke"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(STROKE_SHADER)),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pictura-stroke"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pictura-stroke"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("cs_dab"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { pipeline, layout }
    }
}

/// Keeps the wgpu objects that own the raw device alive for the process.
pub(super) struct Devices {
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    resources: OnceLock<ComputeResources>,
    planar: OnceLock<PlanarResources>,
    stroke: OnceLock<StrokeResources>,
    /// The last composite's readback buffer, kept for the next one.
    ///
    /// ponytail: a fresh gigabyte mapping costs ~0.8 s of page faults on the
    /// map before the first byte is read (an integrated GPU's "device" memory
    /// is system RAM), so the largest recent readback stays resident; release
    /// it under memory pressure if that ever matters.
    readback: Mutex<Option<wgpu::Buffer>>,
    /// Layer sources and coverages kept between composites (`resident.rs`).
    resident: Mutex<Resident>,
}

impl Devices {
    fn resources(&'static self) -> &'static ComputeResources {
        self.resources
            .get_or_init(|| ComputeResources::new(&self.device))
    }

    pub(super) fn planar_resources(&'static self) -> &'static PlanarResources {
        self.planar
            .get_or_init(|| PlanarResources::new(&self.device))
    }

    pub(super) fn stroke_resources(&'static self) -> &'static StrokeResources {
        self.stroke
            .get_or_init(|| StrokeResources::new(&self.device))
    }
}

async fn request_device() -> Option<Devices> {
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
    if !accelerates(adapter.get_info().device_type)
        || adapter.limits().max_storage_buffers_per_shader_stage < 3
    {
        return None;
    }
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("pictura-render-gpu"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        })
        .await
        .ok()?;
    device.set_device_lost_callback(|reason, message| {
        LOST.store(true, std::sync::atomic::Ordering::Relaxed);
        eprintln!("pictura: GPU device lost ({reason:?}: {message}); using the CPU from now on");
    });
    Some(Devices {
        _instance: instance,
        _adapter: adapter,
        device,
        queue,
        resources: OnceLock::new(),
        planar: OnceLock::new(),
        stroke: OnceLock::new(),
        readback: Mutex::new(None),
        resident: Mutex::new(Resident::default()),
    })
}

/// Whether an adapter of this type can speed compositing up. A software
/// Vulkan device (Mesa's lavapipe/llvmpipe, found on machines without a GPU)
/// emulates the GPU on the CPU and measured slower than the CPU compositor
/// itself, so it is treated as no GPU at all.
pub(super) fn accelerates(device_type: wgpu::DeviceType) -> bool {
    device_type != wgpu::DeviceType::Cpu
}

/// Uploads served from the resident cache, for the tests.
#[cfg(test)]
pub(super) static RESIDENT_HITS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Set once the driver reports the shared device lost (a reset after a hung
/// job, for one). A lost device never comes back, so every later caller sees
/// [`GpuError::Unavailable`] and takes the CPU path.
static LOST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn device_lost() -> bool {
    LOST.load(std::sync::atomic::Ordering::Relaxed)
}

pub(super) fn devices() -> Result<&'static Devices, GpuError> {
    static DEVICES: OnceLock<Option<Devices>> = OnceLock::new();
    if device_lost() {
        return Err(GpuError::Unavailable);
    }
    DEVICES
        .get_or_init(|| pollster::block_on(request_device()))
        .as_ref()
        .ok_or(GpuError::Unavailable)
}

/// The process-wide cached device and queue, shared by the compositor and the
/// M27 filter kernels. `None` when no Vulkan adapter is usable, or when the
/// device failed the compositor's one-time CPU-oracle check (a device that
/// miscompiles the compositor miscompiles the filter kernels too). Cheap to
/// call repeatedly and never panics.
pub(crate) fn shared_device() -> Option<(&'static wgpu::Device, &'static wgpu::Queue)> {
    if !super::gpu_available() {
        return None;
    }
    devices().ok().map(|d| (&d.device, &d.queue))
}

/// 2-D compute grid for `n` single-invocation items at 64 per workgroup.
///
/// `gx` covers one row (capped at the device's per-dimension limit) and `gy`
/// stacks the remaining rows, so `gx * 64 * gy >= n`. Returns `None` when the
/// `gx * gy` workgroup product itself exceeds the limit — the device cannot
/// address that many workgroups (65535² × 64 ≈ 2.8×10¹⁴ items). `gx` is always
/// ≥ 1; a zero-item dispatch yields `gy == 0`.
///
/// ponytail: with `n: u32` and the standard 65535 limit, `limit² × 64` exceeds
/// `u32::MAX`, so the product guard can never fire on a compliant device; it
/// exists for adapters that report a smaller `max_compute_workgroups_per_dimension`.
pub(crate) fn grid_2d(n: u32, max_per_dim: u32) -> Option<(u32, u32)> {
    let gx = n.div_ceil(64).min(max_per_dim).max(1);
    let gy = u64::from(n).div_ceil(u64::from(gx) * 64);
    (gy <= u64::from(max_per_dim)).then_some((gx, gy as u32))
}

pub(super) struct Gpu {
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    res: &'static ComputeResources,
    planar: &'static PlanarResources,
    params: wgpu::Buffer,
    /// Region origin in document pixels.
    x0: u32,
    y0: u32,
    /// Region dimensions; the canvas is `w * h` packed words.
    w: u32,
    h: u32,
    pub(super) n: u32,
}

impl Gpu {
    pub(super) fn new(x0: u32, y0: u32, w: u32, h: u32) -> Result<Self, GpuError> {
        let shared = devices()?;
        let device = shared.device.clone();
        let queue = shared.queue.clone();
        let n = u64::from(w) * u64::from(h);
        let limits = device.limits();
        let bytes = n.saturating_mul(4);
        let limit = u64::from(limits.max_compute_workgroups_per_dimension);
        if bytes > limits.max_storage_buffer_binding_size
            || bytes > limits.max_buffer_size
            || n > limit.saturating_mul(limit).saturating_mul(64)
            || n > u64::from(u32::MAX)
        {
            return Err(GpuError::TooLarge);
        }
        let n = n as u32;
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pictura-blend-params"),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Self {
            device,
            queue,
            res: shared.resources(),
            planar: shared.planar_resources(),
            params,
            x0,
            y0,
            w,
            h,
            n,
        })
    }

    /// A mappable readback buffer of at least `size` bytes: the kept one when
    /// it is large enough, else a fresh one.
    fn readback_buffer(&self, size: u64) -> wgpu::Buffer {
        let kept = devices()
            .ok()
            .and_then(|d| d.readback.lock().ok().and_then(|mut k| k.take()))
            .filter(|b| b.size() >= size);
        kept.unwrap_or_else(|| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("pictura-readback"),
                size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        })
    }

    pub(super) fn region(&self) -> Region {
        Region {
            x0: self.x0,
            y0: self.y0,
            w: self.w,
            h: self.h,
        }
    }

    pub(super) fn zero_canvas(&self) -> wgpu::Buffer {
        // ponytail: clear_buffer (GPU memset) over a host `vec![0; n*4]` upload;
        // COPY_DST is required by clear_buffer and the readback's COPY_SRC.
        let canvas = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pictura-canvas"),
            size: u64::from(self.n) * 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("pictura-canvas-clear"),
            });
        encoder.clear_buffer(&canvas, 0, None);
        self.queue.submit(Some(encoder.finish()));
        canvas
    }

    /// Assemble the source for one pixel layer over its layer-rect intersection
    /// with the region, matching `composite_pixels`: planar 8-bit channel
    /// planes, grayscale replicating channel 0 (one colour plane), and straight
    /// alpha defaulting to 255 when absent. The shader samples by document
    /// pixel; outside the intersection the source alpha is 0.
    pub(super) fn build_source(
        &self,
        layer: &Layer,
        doc: &Document,
    ) -> Option<(wgpu::Buffer, SrcLayout)> {
        let g = source_geom(self.region(), layer, doc)?;
        let key = source_key(self.region(), layer, g.gray);
        let buffer = self.kept_or_upload(key, || {
            self.upload_with("pictura-src", g.planes * g.n, source_writer(layer, &g))
        });
        Some((buffer, g.layout()))
    }

    /// The upload kept under `key`, or `upload()`'s, kept for next time. A
    /// `None` key (an unstamped plane) always uploads and keeps nothing.
    fn kept_or_upload(
        &self,
        key: Option<UploadKey>,
        upload: impl FnOnce() -> wgpu::Buffer,
    ) -> wgpu::Buffer {
        let resident = || devices().ok().and_then(|d| d.resident.lock().ok());
        if let Some(buffer) = key
            .as_ref()
            .and_then(|k| resident().and_then(|mut r| r.get(k)))
        {
            #[cfg(test)]
            RESIDENT_HITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return buffer;
        }
        let buffer = upload();
        if let (Some(key), Some(mut r)) = (key, resident()) {
            r.put(key, buffer.clone());
        }
        buffer
    }

    /// A storage buffer of `len` bytes (padded to a word) whose bytes `write`
    /// produces straight into the queue's staging memory, a [`RUN`] per staging
    /// write, the runs in parallel: a large layer is never first assembled
    /// into a host buffer and then copied again.
    fn upload_with(
        &self,
        label: &str,
        len: usize,
        write: impl Fn(usize, wgpu::WriteOnly<'_, [u8]>) + Sync,
    ) -> wgpu::Buffer {
        use rayon::prelude::*;
        let size = padded(len).max(4);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Every run lies inside the fresh buffer, so a refused write means the
        // device is lost: leave the buffer unwritten, and `read_canvas` fails.
        let views: Option<Vec<_>> = (0..size)
            .step_by(RUN)
            .map(|at| {
                let bytes =
                    wgpu::BufferSize::new(RUN.min(size - at) as u64).expect("a run is never empty");
                let view = self.queue.write_buffer_with(&buffer, at as u64, bytes)?;
                Some((at, view))
            })
            .collect();
        views
            .unwrap_or_default()
            .into_par_iter()
            .for_each(|(at, mut view)| write(at, view.slice(..)));
        buffer
    }

    /// Per-region-pixel mask coverage as an 8-bit plane, reusing the CPU
    /// `mask_alpha`. A group's inner buffer and an adjustment layer act across
    /// the whole region; a pixel layer only over its layer-rect intersection
    /// with the region (outside it the source alpha is 0, so the coverage is
    /// irrelevant).
    ///
    /// ponytail: the plane is region-sized even for a rect-scoped layer; bound
    /// it by the rect if a many-small-layers mask profile ever shows up.
    pub(super) fn build_mask(&self, layer: &Layer) -> wgpu::Buffer {
        let region = self.region();
        let len = region.w as usize * region.h as usize;
        self.kept_or_upload(mask_key(region, layer), || {
            self.upload_with("pictura-mask", len, mask_writer(region, layer))
        })
    }

    pub(super) fn composite_layer(&self, canvas: &wgpu::Buffer, layer: &Layer, doc: &Document) {
        if !layer.visible {
            return;
        }
        if layer.is_group {
            // True pass-through: recurse onto the running canvas (matches the
            // CPU exactness condition).
            if matches!(layer.blend, BlendMode::PassThrough)
                && layer.opacity == 255
                && layer.mask.is_none()
            {
                for child in &layer.children {
                    self.composite_layer(canvas, child, doc);
                }
                return;
            }
            let inner = self.zero_canvas();
            for child in &layer.children {
                self.composite_layer(&inner, child, doc);
            }
            let mask = self.build_mask(layer);
            // The inner canvas is already packed 8-bit RGBA: bind it directly
            // as the source, no `f32` re-materialization.
            self.dispatch(
                canvas,
                &inner,
                &mask,
                PACKED_SRC,
                layer.blend,
                layer.opacity,
                255,
                NO_ADJ,
            );
            return;
        }
        if let Some(data) = &layer.adjustment {
            // `check_supported` already rejected every adjustment that has no GPU
            // kind, so this only runs for the five supported kinds.
            if let Some(adj) = decode_adjustment(data).and_then(|a| adjustment_params(&a)) {
                let mask = self.build_mask(layer);
                // The shader ignores `src` in the adjustment branch; the mask
                // buffer stands in for it rather than aliasing the read-write
                // canvas binding.
                self.dispatch(
                    canvas,
                    &mask,
                    &mask,
                    PACKED_SRC,
                    layer.blend,
                    layer.opacity,
                    layer.fill,
                    adj,
                );
            }
            return;
        }
        if let Some((src, layout)) = self.build_source(layer, doc) {
            let mask = self.build_mask(layer);
            self.dispatch(
                canvas,
                &src,
                &mask,
                layout,
                layer.blend,
                layer.opacity,
                layer.fill,
                NO_ADJ,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn dispatch(
        &self,
        canvas: &wgpu::Buffer,
        src: &wgpu::Buffer,
        mask: &wgpu::Buffer,
        layout: SrcLayout,
        mode: BlendMode,
        opacity: u8,
        fill: u8,
        adj: (u32, i32, i32, i32),
    ) {
        let flags = u32::from(layout.packed) | (u32::from(layout.gray) << 1);
        let (gx, gy) = grid_2d(
            self.n,
            self.device.limits().max_compute_workgroups_per_dimension,
        )
        .expect("Gpu::new rejected a canvas past the 2-D workgroup limit");
        let params = [
            mode_id(mode).to_le_bytes(),
            (opacity as f32 / 255.0).to_le_bytes(),
            (fill as f32 / 255.0).to_le_bytes(),
            self.n.to_le_bytes(),
            adj.0.to_le_bytes(),
            adj.1.to_le_bytes(),
            adj.2.to_le_bytes(),
            adj.3.to_le_bytes(),
            flags.to_le_bytes(),
            layout.x0.to_le_bytes(),
            layout.y0.to_le_bytes(),
            layout.w.to_le_bytes(),
            layout.h.to_le_bytes(),
            self.x0.to_le_bytes(),
            self.y0.to_le_bytes(),
            self.w.to_le_bytes(),
            (gx * 64).to_le_bytes(),
        ]
        .concat();
        self.queue.write_buffer(&self.params, 0, &params);

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pictura-blend"),
            layout: &self.res.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: canvas.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: src.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: mask.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.params.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("pictura-blend"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("pictura-blend"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.res.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(gx, gy, 1);
        }
        self.queue.submit(Some(encoder.finish()));
    }

    pub(super) fn read_canvas(&self, canvas: &wgpu::Buffer) -> Result<PixelBuffer, GpuError> {
        // De-interleave on the GPU: the planar pass writes four byte planes into
        // one storage buffer, so the host copies plane slices instead of
        // gathering a channel per pixel. `pw` words per plane (word-aligned by
        // construction); bytes past `n` in the last word of a plane are padding.
        let n = self.n;
        let pw = n.div_ceil(4);
        let size = u64::from(pw) * 16;
        let planar = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pictura-planar"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pictura-planar-params"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (gx, gy) = grid_2d(
            pw,
            self.device.limits().max_compute_workgroups_per_dimension,
        )
        .expect("Gpu::new rejected a canvas past the 2-D workgroup limit");
        // Whole-buffer addresses: bx = by = 0, bw = lw = canvas width.
        let params_data = [
            n.to_le_bytes(),
            pw.to_le_bytes(),
            (gx * 64).to_le_bytes(),
            0u32.to_le_bytes(),
            0u32.to_le_bytes(),
            self.w.to_le_bytes(),
            self.w.to_le_bytes(),
            0u32.to_le_bytes(),
        ]
        .concat();
        self.queue.write_buffer(&params, 0, &params_data);

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pictura-planar"),
            layout: &self.planar.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: canvas.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: planar.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params.as_entire_binding(),
                },
            ],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("pictura-planar"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("pictura-planar"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.planar.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(gx, gy, 1);
        }
        self.queue.submit(Some(encoder.finish()));

        let staging = self.readback_buffer(size);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("pictura-readback"),
            });
        encoder.copy_buffer_to_buffer(&planar, 0, &staging, 0, size);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..size);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv()
            .map_err(|_| GpuError::Readback)?
            .map_err(|_| GpuError::Readback)?;
        if device_lost() {
            return Err(GpuError::Readback);
        }

        // Each plane is `pw` words; copy exactly `n` bytes of it into the
        // planar `PixelBuffer`, ignoring the tail padding.
        let mapped = slice.get_mapped_range().map_err(|_| GpuError::Readback)?;
        let plane = pw as usize * 4;
        let n = n as usize;
        let src: &[u8] = &mapped;
        // One allocation straight from zeroed pages, each plane copied in one
        // pass: a 267-megapixel readback is a gigabyte. Its pages are faulted
        // in on every core first; the copy itself stays on one thread, since
        // parallel reads of the mapped range measured four times slower.
        let data = pictura_core::Plane::build(4 * n, |out| {
            use rayon::prelude::*;
            if n == 0 {
                return;
            }
            out.par_chunks_mut(RUN).for_each(|run| run.fill(0));
            for (c, dst) in out.chunks_mut(n).enumerate() {
                dst.copy_from_slice(&src[c * plane..c * plane + n]);
            }
        });
        let out = PixelBuffer {
            width: self.w,
            height: self.h,
            channels: 4,
            data,
        };
        drop(mapped);
        staging.unmap();
        if let Ok(mut kept) = devices()?.readback.lock() {
            *kept = Some(staging);
        }
        Ok(out)
    }

    /// De-interleave packed RGBA8 readback into the planar straight-alpha
    /// `PixelBuffer`, identical in shape to `composite_rgba`. Each LE `u32`
    /// unpacked and masked into four channels is a pure byte permutation, so
    /// walking 4-byte chunks writes the same bytes. The live readback now
    /// planarizes on the GPU; this host gather stays for the profile micro-
    /// benchmark and as the retained reference.
    #[cfg(test)]
    pub(super) fn to_pixel_buffer(&self, packed: &[u8]) -> PixelBuffer {
        let plane = self.n as usize;
        let mut out = PixelBuffer::new(self.w, self.h, 4);
        for (i, px) in packed.as_chunks::<4>().0.iter().take(plane).enumerate() {
            out.data[i] = px[0];
            out.data[plane + i] = px[1];
            out.data[2 * plane + i] = px[2];
            out.data[3 * plane + i] = px[3];
        }
        out
    }
}

/// How the shader should read the source binding for one dispatch.
#[derive(Clone, Copy)]
pub(super) struct SrcLayout {
    pub(super) x0: u32,
    pub(super) y0: u32,
    pub(super) w: u32,
    pub(super) h: u32,
    pub(super) gray: bool,
    pub(super) packed: bool,
}

/// A group's inner canvas: one packed RGBA word per region pixel.
const PACKED_SRC: SrcLayout = SrcLayout {
    x0: 0,
    y0: 0,
    w: 0,
    h: 0,
    gray: false,
    packed: true,
};
