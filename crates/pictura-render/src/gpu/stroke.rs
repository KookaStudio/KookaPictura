//! GPU dab rasterization for the live paint stroke.
//!
//! A [`GpuStroke`] keeps one target layer's pixels resident as interleaved
//! RGBA8 and applies each dab with a compute dispatch that accumulates coverage
//! and composites it into the layer in one pass, matching `pictura-paint`'s
//! `Stroke::sample` + `Stencil::composite` for the Normal and Clear modes within
//! ±1 LSB. The CPU `Stroke` remains the oracle and the fallback; a stack or mode
//! this type does not model is not passed to it.

use pictura_core::PsdRect;

use super::backend::{grid_2d, PlanarResources, StrokeResources};
use super::{devices, GpuError};

/// The paint modes the GPU dab path models. Anything else is CPU-only.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GpuPaintMode {
    Normal,
    Clear,
}

/// The stroke-constant tip and accumulation inputs, in the same terms
/// `pictura_paint`'s `TipParams` and `StrokeConfig` use: `radius`/`round_radius`
/// are in pixels, `color` is straight alpha in 0..=1, `flow`/`opacity` are 0..=1.
#[derive(Clone, Copy, Debug)]
pub struct GpuStrokeParams {
    pub radius: f32,
    pub round_radius: f32,
    pub sin_t: f32,
    pub cos_t: f32,
    pub core: f32,
    pub denom: f32,
    pub flow: f32,
    pub opacity: f32,
    pub color: [f32; 4],
    pub aliased: bool,
    pub flip_x: bool,
    pub flip_y: bool,
    pub mode: GpuPaintMode,
    /// 4 for an RGBA layer, 1 for a single-channel mask. A 1-channel stroke
    /// blends only byte 0 and leaves the other channels untouched.
    pub channels: u32,
}

/// The `Dab` uniform's byte size; 24 words, padded to a 16-byte multiple.
const PARAM_BYTES: usize = 96;

/// One target layer of a stroke, resident on the GPU.
pub struct GpuStroke {
    device: wgpu::Device,
    queue: wgpu::Queue,
    res: &'static StrokeResources,
    planar: &'static PlanarResources,
    layer: wgpu::Buffer,
    /// The pre-stroke pixels, immutable for the stroke. Every changed coverage
    /// recomposites from this, so overlapping dabs match the CPU oracle instead
    /// of compounding on the GPU's own earlier output.
    base: wgpu::Buffer,
    coverage: wgpu::Buffer,
    params: wgpu::Buffer,
    width: u32,
    height: u32,
    cfg: GpuStrokeParams,
}

impl GpuStroke {
    /// Whether a GPU stroke can run at all.
    pub fn available() -> bool {
        super::gpu_available()
    }

    /// Seed a stroke from the target layer's interleaved RGBA8 pixels
    /// (`width * height * 4` bytes) and its stroke-constant parameters.
    pub fn new(
        width: u32,
        height: u32,
        rgba: &[u8],
        cfg: GpuStrokeParams,
    ) -> Result<Self, GpuError> {
        let n = u64::from(width) * u64::from(height);
        if width == 0 || height == 0 || rgba.len() < (n as usize) * 4 {
            return Err(GpuError::TooLarge);
        }
        // The app seeds via `new().ok()`, not `available()`, so the gate must
        // live here too: a device that miscompiles the compositor miscompiles
        // `STROKE_SHADER`, and a GPU stroke commits with no CPU replay.
        if !super::gpu_available() {
            return Err(GpuError::Unavailable);
        }
        let shared = devices()?;
        let device = shared.device.clone();
        let queue = shared.queue.clone();
        let planar = shared.planar_resources();
        let limits = device.limits();
        let bytes = n * 4;
        if bytes > limits.max_storage_buffer_binding_size
            || bytes > limits.max_buffer_size
            || n > u64::from(u32::MAX)
        {
            return Err(GpuError::TooLarge);
        }
        let store = wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST;
        let layer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pictura-stroke-layer"),
            size: bytes,
            usage: store,
            mapped_at_creation: false,
        });
        queue.write_buffer(&layer, 0, &rgba[..(n as usize) * 4]);
        let base = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pictura-stroke-base"),
            size: bytes,
            usage: store,
            mapped_at_creation: false,
        });
        // One word per pixel: a packed byte plane would be written by four
        // invocations with a read-modify-write and race.
        let coverage = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pictura-stroke-coverage"),
            size: bytes,
            usage: store,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("pictura-stroke-clear"),
        });
        encoder.copy_buffer_to_buffer(&layer, 0, &base, 0, bytes);
        encoder.clear_buffer(&coverage, 0, None);
        queue.submit(Some(encoder.finish()));
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pictura-stroke-params"),
            size: PARAM_BYTES as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Self {
            device,
            queue,
            res: shared.stroke_resources(),
            planar,
            layer,
            base,
            coverage,
            params,
            width,
            height,
            cfg,
        })
    }

    /// Apply one dab centered at layer-local `(cx, cy)`, returning the
    /// layer-local rectangle it changed with its pixels as four planar byte
    /// planes, or `None` when the dab falls outside the layer. `plane_stride` is
    /// the bytes from one plane's start to the next; each plane is row-major
    /// `width * height` of the rectangle, padded up to a word.
    pub fn dab(&mut self, cx: f32, cy: f32) -> Result<Option<(PsdRect, usize, Vec<u8>)>, GpuError> {
        let r = self.cfg.radius;
        let x0 = ((cx - r).floor() as i64).clamp(0, self.width as i64) as u32;
        let y0 = ((cy - r).floor() as i64).clamp(0, self.height as i64) as u32;
        let x1 = ((cx + r).ceil() as i64).clamp(0, self.width as i64) as u32;
        let y1 = ((cy + r).ceil() as i64).clamp(0, self.height as i64) as u32;
        if x1 <= x0 || y1 <= y0 {
            return Ok(None);
        }
        let (bw, bh) = (x1 - x0, y1 - y0);
        let (gx, gy) = grid_2d(
            bw * bh,
            self.device.limits().max_compute_workgroups_per_dimension,
        )
        .ok_or(GpuError::TooLarge)?;
        // A bbox past the per-dimension workgroup limit spans several dispatch
        // rows; the shader needs the x stride (written by `write_params`) to
        // walk them.
        self.write_params(cx, cy, x0, y0, bw, bh);
        self.dispatch(gx, gy)?;
        let (plane_stride, planes) = self.read_rect_planar(x0, y0, bw, bh)?;
        Ok(Some((
            PsdRect {
                left: x0 as i32,
                top: y0 as i32,
                right: x1 as i32,
                bottom: y1 as i32,
            },
            plane_stride,
            planes,
        )))
    }

    /// Read the whole resident layer back as interleaved RGBA8.
    pub fn layer_rgba(&self) -> Result<Vec<u8>, GpuError> {
        self.read_rect(0, 0, self.width, self.height)
    }

    fn write_params(&self, cx: f32, cy: f32, bx: u32, by: u32, bw: u32, bh: u32) {
        let mut buf = [0u8; PARAM_BYTES];
        let put = |b: &mut [u8], at: usize, v: f32| {
            b[at..at + 4].copy_from_slice(&v.to_le_bytes());
        };
        let c = self.cfg;
        put(&mut buf, 0, cx);
        put(&mut buf, 4, cy);
        put(&mut buf, 8, c.radius);
        put(&mut buf, 12, c.round_radius);
        put(&mut buf, 16, c.sin_t);
        put(&mut buf, 20, c.cos_t);
        put(&mut buf, 24, c.core);
        put(&mut buf, 28, c.denom);
        put(&mut buf, 32, c.flow);
        put(&mut buf, 36, c.opacity);
        put(&mut buf, 40, bx as f32);
        put(&mut buf, 44, by as f32);
        put(&mut buf, 48, bw as f32);
        put(&mut buf, 52, bh as f32);
        put(&mut buf, 56, self.width as f32);
        put(&mut buf, 60, if c.aliased { 1.0 } else { 0.0 });
        for (k, v) in c.color.iter().enumerate() {
            put(&mut buf, 64 + k * 4, *v);
        }
        put(
            &mut buf,
            80,
            match c.mode {
                GpuPaintMode::Normal => 0.0,
                GpuPaintMode::Clear => 1.0,
            },
        );
        let flip = if c.flip_x { 1.0 } else { 0.0 } + if c.flip_y { 2.0 } else { 0.0 };
        put(&mut buf, 84, flip);
        // The dispatched x extent, so the shader can walk `gid.y` when the
        // bbox spans more than one dispatch row.
        let stride = grid_2d(
            bw * bh,
            self.device.limits().max_compute_workgroups_per_dimension,
        )
        .map_or(0, |(gx, _)| gx * 64);
        put(&mut buf, 88, stride as f32);
        put(&mut buf, 92, c.channels as f32);
        self.queue.write_buffer(&self.params, 0, &buf);
    }

    fn dispatch(&self, gx: u32, gy: u32) -> Result<(), GpuError> {
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pictura-stroke"),
            layout: &self.res.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.layer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.coverage.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.base.as_entire_binding(),
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
                label: Some("pictura-stroke"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("pictura-stroke"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.res.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(gx, gy, 1);
        }
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    /// Copy `w × h` of the resident layer at `(x0, y0)` back as interleaved
    /// RGBA8. Rows are copied one at a time: the source rows are `width * 4`
    /// apart, so the rectangle is not contiguous in the layer buffer.
    fn read_rect(&self, x0: u32, y0: u32, w: u32, h: u32) -> Result<Vec<u8>, GpuError> {
        let size = u64::from(w) * u64::from(h) * 4;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pictura-stroke-readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("pictura-stroke-readback"),
            });
        let row = u64::from(w) * 4;
        for y in 0..h {
            let src = (u64::from(y0 + y) * u64::from(self.width) + u64::from(x0)) * 4;
            encoder.copy_buffer_to_buffer(&self.layer, src, &staging, u64::from(y) * row, row);
        }
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv()
            .map_err(|_| GpuError::Readback)?
            .map_err(|_| GpuError::Readback)?;
        let mapped = slice.get_mapped_range().map_err(|_| GpuError::Readback)?;
        let out = mapped.to_vec();
        drop(mapped);
        staging.unmap();
        Ok(out)
    }

    /// De-interleave `w × h` of the resident layer at `(x0, y0)` on the GPU and
    /// return its four planar byte planes plus the plane stride. The host then
    /// patches the working layer with row copies instead of a per-pixel gather.
    fn read_rect_planar(
        &self,
        x0: u32,
        y0: u32,
        w: u32,
        h: u32,
    ) -> Result<(usize, Vec<u8>), GpuError> {
        let n = w * h;
        let pw = n.div_ceil(4).max(1);
        let plane_bytes = u64::from(pw) * 4;
        let size = plane_bytes * 4;
        let planar = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pictura-stroke-planar"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pictura-stroke-planar-params"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (gx, gy) = grid_2d(
            pw,
            self.device.limits().max_compute_workgroups_per_dimension,
        )
        .ok_or(GpuError::TooLarge)?;
        let params_data = [
            n.to_le_bytes(),
            pw.to_le_bytes(),
            (gx * 64).to_le_bytes(),
            x0.to_le_bytes(),
            y0.to_le_bytes(),
            w.to_le_bytes(),
            self.width.to_le_bytes(),
            0u32.to_le_bytes(),
        ]
        .concat();
        self.queue.write_buffer(&params, 0, &params_data);

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pictura-stroke-planar"),
            layout: &self.planar.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.layer.as_entire_binding(),
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
                label: Some("pictura-stroke-planar"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("pictura-stroke-planar"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.planar.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(gx, gy, 1);
        }
        self.queue.submit(Some(encoder.finish()));

        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pictura-stroke-planar-readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("pictura-stroke-planar-readback"),
            });
        encoder.copy_buffer_to_buffer(&planar, 0, &staging, 0, size);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv()
            .map_err(|_| GpuError::Readback)?
            .map_err(|_| GpuError::Readback)?;
        let mapped = slice.get_mapped_range().map_err(|_| GpuError::Readback)?;
        let out = mapped.to_vec();
        drop(mapped);
        staging.unmap();
        Ok((plane_bytes as usize, out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LockFlags,
    };
    use pictura_paint::{spacing::SpacingMode, Stroke, StrokeSample};

    /// The GPU stroke reaches the device through `devices()`, not
    /// `shared_device()`, and the app seeds it via `new().ok()`; both must
    /// consult the compositor validation so a miscompiled `STROKE_SHADER` cannot
    /// patch the document with no CPU replay.
    #[test]
    fn the_stroke_gate_follows_the_compositor_validation() {
        assert_eq!(GpuStroke::available(), crate::gpu::gpu_available());
        if GpuStroke::available() {
            return;
        }
        let cfg = GpuStrokeParams {
            radius: 4.0,
            round_radius: 4.0,
            sin_t: 0.0,
            cos_t: 1.0,
            core: 0.5,
            denom: 1.0,
            flow: 1.0,
            opacity: 1.0,
            color: [1.0, 0.0, 0.0, 1.0],
            aliased: false,
            flip_x: false,
            flip_y: false,
            mode: GpuPaintMode::Normal,
            channels: 4,
        };
        assert!(matches!(
            GpuStroke::new(2, 2, &[0u8; 16], cfg),
            Err(GpuError::Unavailable)
        ));
    }

    fn layer_doc(w: u32, h: u32, rgba: (u8, u8, u8, u8)) -> Document {
        let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
        let n = (w * h) as usize;
        let (r, g, b, a) = rgba;
        doc.layers.push(Layer {
            name: "px".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: h as i32,
                right: w as i32,
            },
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            lock: LockFlags::default(),
            color: ColorLabel::None,
            clipping: false,
            visible: true,
            mask: None,
            adjustment: None,
            channels: vec![
                Channel {
                    id: 0,
                    data: vec![r; n].into(),
                },
                Channel {
                    id: 1,
                    data: vec![g; n].into(),
                },
                Channel {
                    id: 2,
                    data: vec![b; n].into(),
                },
                Channel {
                    id: -1,
                    data: vec![a; n].into(),
                },
            ],
            children: Vec::new(),
            is_group: false,
            background: false,
            ..Default::default()
        });
        doc
    }

    fn interleaved(layer: &Layer, w: u32, h: u32) -> Vec<u8> {
        let n = (w * h) as usize;
        let at = |id: i16| {
            layer
                .channels
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.data.as_slice())
                .unwrap_or(&[])
        };
        let (r, g, b, a) = (at(0), at(1), at(2), at(-1));
        let mut out = vec![0u8; n * 4];
        for i in 0..n {
            out[i * 4] = r.get(i).copied().unwrap_or(0);
            out[i * 4 + 1] = g.get(i).copied().unwrap_or(0);
            out[i * 4 + 2] = b.get(i).copied().unwrap_or(0);
            out[i * 4 + 3] = a.get(i).copied().unwrap_or(255);
        }
        out
    }

    fn params_from(cfg: &pictura_paint::StrokeConfig) -> GpuStrokeParams {
        let radius = cfg.diameter as f32 * 0.5;
        let round = if cfg.roundness == 0 {
            1.0
        } else {
            cfg.roundness as f32 / 100.0
        };
        let hardness = cfg.hardness as f32 / 100.0;
        let core = hardness * (1.0 - (1.0 / radius).min(0.5));
        let theta = -(cfg.angle_deg as f32).to_radians();
        GpuStrokeParams {
            radius,
            round_radius: radius * round,
            sin_t: theta.sin(),
            cos_t: theta.cos(),
            core,
            denom: (1.0 - core).max(f32::EPSILON),
            flow: cfg.flow as f32 / 100.0,
            opacity: cfg.opacity as f32 / 100.0,
            color: [
                cfg.color.r as f32 / 255.0,
                cfg.color.g as f32 / 255.0,
                cfg.color.b as f32 / 255.0,
                cfg.color.a as f32 / 255.0,
            ],
            aliased: cfg.aliased,
            flip_x: cfg.flip_x,
            flip_y: cfg.flip_y,
            mode: GpuPaintMode::Normal,
            channels: 4,
        }
    }

    #[test]
    fn the_dab_shader_walks_every_dispatch_row() {
        // A bbox larger than `max_compute_workgroups_per_dimension * 64` (the
        // standard 65535 limit, ~4.19M px — a ~2500 px brush on a 4000² canvas)
        // spans several dispatched y rows. The shader must fold `gid.y` in with
        // the x stride; a bare `gid.x` repeats the first row and leaves the
        // bbox tail uncomposited.
        assert!(
            crate::gpu::shader::STROKE_SHADER.contains("gid.x + gid.y * stride"),
            "the dab shader must walk gid.y, not only gid.x"
        );
        let n = 4000u32 * 4000;
        let (gx, gy) = crate::gpu::backend::grid_2d(n, 65_535).expect("4000² fits the 2-D grid");
        assert!(gy > 1, "this bbox must exercise more than one dispatch row");
        assert!(
            u64::from(gx * 64) * u64::from(gy) >= u64::from(n),
            "the stride walk must cover every bbox pixel"
        );
    }

    #[test]
    #[ignore = "GPU stroke parity; run explicitly with --ignored"]
    fn gpu_stroke_matches_the_cpu_oracle() {
        if !GpuStroke::available() {
            eprintln!("no GPU adapter; skipping");
            return;
        }
        let (w, h) = (64u32, 64u32);
        let base = (100u8, 120, 140, 200);
        let cfg = pictura_paint::StrokeConfig {
            color: pictura_paint::Rgba {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
            diameter: 32,
            hardness: 60,
            roundness: 100,
            opacity: 80,
            flow: 70,
            spacing: SpacingMode::Fixed(50),
            ..Default::default()
        };
        // A dense diagonal: the placer steps every 16 px, so consecutive dabs
        // overlap heavily and each pixel is covered by several.
        let dabs: Vec<(f32, f32)> = (0..8)
            .map(|i| (16.0 + i as f32 * 5.0, 16.0 + i as f32 * 5.0))
            .collect();

        // CPU oracle.
        let mut doc = layer_doc(w, h, base);
        let mut stroke = Stroke::begin_at(&doc, "0", cfg).expect("stroke begins");
        for &(x, y) in &dabs {
            stroke.sample(
                &mut doc,
                StrokeSample {
                    x,
                    y,
                    pressure: 1.0,
                },
            );
        }
        stroke.finish().expect("painted");
        let cpu = interleaved(&doc.layers[0], w, h);

        // GPU stroke, seeded from the same pixels and driven through the same
        // dab placer the exact stroke uses, so both place identical dabs.
        let seed = interleaved(&layer_doc(w, h, base).layers[0], w, h);
        let mut gpu = GpuStroke::new(w, h, &seed, params_from(&cfg)).expect("gpu stroke");
        let roundtrip = gpu.layer_rgba().expect("readback");
        assert_eq!(roundtrip, seed, "the seeded layer round-trips unchanged");
        let mut placer = pictura_paint::spacing::DabPlacer::new(cfg.spacing, cfg.diameter as f32);
        let mut placed = Vec::new();
        for &(x, y) in &dabs {
            placed.clear();
            placer.feed(x, y, &mut placed);
            for (cx, cy) in &placed {
                let (rect, stride, planes) = gpu
                    .dab(*cx, *cy)
                    .expect("gpu dab")
                    .expect("a centred dab changes pixels");
                // The planar readback must de-interleave the resident layer
                // exactly, so the host patch sees the same bytes.
                let whole = gpu.layer_rgba().expect("readback");
                let bw = rect.width() as usize;
                for row in 0..rect.height() as usize {
                    for col in 0..bw {
                        let sx = (rect.top as usize + row) * w as usize + rect.left as usize + col;
                        let k = row * bw + col;
                        assert_eq!(
                            [
                                planes[k],
                                planes[stride + k],
                                planes[2 * stride + k],
                                planes[3 * stride + k],
                            ],
                            [
                                whole[sx * 4],
                                whole[sx * 4 + 1],
                                whole[sx * 4 + 2],
                                whole[sx * 4 + 3],
                            ],
                            "planar readback matches the resident layer at ({col},{row})"
                        );
                    }
                }
            }
        }
        let got = gpu.layer_rgba().expect("readback");

        let mut worst = 0i32;
        for (a, b) in cpu.iter().zip(got.iter()) {
            worst = worst.max((i32::from(*a) - i32::from(*b)).abs());
        }
        assert!(
            worst <= 1,
            "GPU stroke differs from the CPU oracle by {worst} LSB"
        );
    }

    #[test]
    #[ignore = "GPU mask stroke parity; run explicitly with --ignored"]
    fn gpu_mask_stroke_matches_the_cpu_oracle() {
        if !GpuStroke::available() {
            eprintln!("no GPU adapter; skipping");
            return;
        }
        let (w, h) = (64u32, 64u32);
        let base = 130u8;
        let cfg = pictura_paint::StrokeConfig {
            color: pictura_paint::Rgba {
                r: 200,
                g: 200,
                b: 200,
                a: 255,
            },
            diameter: 32,
            hardness: 60,
            roundness: 100,
            opacity: 80,
            flow: 70,
            spacing: SpacingMode::Fixed(50),
            ..Default::default()
        };
        let dabs: Vec<(f32, f32)> = (0..8)
            .map(|i| (16.0 + i as f32 * 5.0, 16.0 + i as f32 * 5.0))
            .collect();
        let flat = |v: u8| Layer {
            name: "mask".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: h as i32,
                right: w as i32,
            },
            channels: vec![Channel {
                id: 0,
                data: vec![v; (w * h) as usize].into(),
            }],
            ..Default::default()
        };

        // CPU oracle on the 1-channel mask document.
        let mut doc = Document::new(w, h, ColorMode::Grayscale, BitDepth::Eight);
        doc.layers.push(flat(base));
        let mut stroke = Stroke::begin_at(&doc, "0", cfg).expect("stroke begins");
        for &(x, y) in &dabs {
            stroke.sample(
                &mut doc,
                StrokeSample {
                    x,
                    y,
                    pressure: 1.0,
                },
            );
        }
        stroke.finish();
        let cpu = doc.layers[0]
            .channels
            .iter()
            .find(|c| c.id == 0)
            .unwrap()
            .data
            .to_vec();

        // GPU 1-channel stroke: the coverage lives in byte 0.
        let n = (w * h) as usize;
        let mut seed = vec![0u8; n * 4];
        for px in seed.as_chunks_mut::<4>().0 {
            px.copy_from_slice(&[base, base, base, 255]);
        }
        let mut params = params_from(&cfg);
        params.channels = 1;
        let mut gpu = GpuStroke::new(w, h, &seed, params).expect("gpu stroke");
        let mut placer = pictura_paint::spacing::DabPlacer::new(cfg.spacing, cfg.diameter as f32);
        let mut placed = Vec::new();
        for &(x, y) in &dabs {
            placed.clear();
            placer.feed(x, y, &mut placed);
            for (cx, cy) in &placed {
                let _ = gpu.dab(*cx, *cy).expect("gpu dab");
            }
        }
        let got = gpu.layer_rgba().expect("readback");
        let mut worst = 0i32;
        for i in 0..n {
            worst = worst.max((i32::from(cpu[i]) - i32::from(got[i * 4])).abs());
        }
        assert!(
            worst <= 1,
            "GPU mask stroke differs from the CPU oracle by {worst} LSB"
        );
    }
}
