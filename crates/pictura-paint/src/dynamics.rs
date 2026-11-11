//! Brush dynamics: the Control inputs and the Shape, Scatter, Color, Transfer,
//! and Pose option sets. Adobe's curves are closed; the model here is the
//! behavioural one documented in `docs/07-color-painting/brush-dynamics.md`.

use crate::Rgba;

/// What drives a dynamics attribute when it is not random jitter alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Control {
    #[default]
    Off,
    Fade(u16),
    PenPressure,
    PenTilt,
    StylusWheel,
    Rotation,
    InitialDirection,
    Direction,
}

impl Control {
    /// The index the C++ bridge and the options JSON use.
    pub fn to_index(self) -> i32 {
        match self {
            Control::Off => 0,
            Control::Fade(_) => 1,
            Control::PenPressure => 2,
            Control::PenTilt => 3,
            Control::StylusWheel => 4,
            Control::Rotation => 5,
            Control::InitialDirection => 6,
            Control::Direction => 7,
        }
    }

    pub fn from_index(index: i32, fade_steps: u16) -> Control {
        match index {
            1 => Control::Fade(fade_steps.clamp(1, 9999)),
            2 => Control::PenPressure,
            3 => Control::PenTilt,
            4 => Control::StylusWheel,
            5 => Control::Rotation,
            6 => Control::InitialDirection,
            7 => Control::Direction,
            _ => Control::Off,
        }
    }

    pub fn fade_steps(self) -> u16 {
        match self {
            Control::Fade(steps) => steps.clamp(1, 9999),
            _ => 1,
        }
    }
}

/// The device values a control reads. Tilt is in degrees, the rest normalized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicsInput {
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub rotation: f32,
    pub wheel: f32,
}

impl Default for DynamicsInput {
    fn default() -> Self {
        Self {
            pressure: 1.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            rotation: 0.0,
            wheel: 0.0,
        }
    }
}

impl DynamicsInput {
    pub fn from_pressure(pressure: f32) -> Self {
        Self {
            pressure: pressure.clamp(0.0, 1.0),
            ..Self::default()
        }
    }

    fn signal(self, control: Control, dab: u32, tangent: f32, initial: f32) -> f32 {
        match control {
            Control::Off => 1.0,
            Control::Fade(steps) => {
                let steps = steps.max(1) as f32;
                (1.0 - dab as f32 / steps).clamp(0.0, 1.0)
            }
            Control::PenPressure => self.pressure.clamp(0.0, 1.0),
            Control::PenTilt => {
                let reach = (self.tilt_x * self.tilt_x + self.tilt_y * self.tilt_y).sqrt();
                (reach / 90.0).clamp(0.0, 1.0)
            }
            Control::StylusWheel => self.wheel.clamp(0.0, 1.0),
            Control::Rotation => self.rotation.rem_euclid(360.0) / 360.0,
            Control::InitialDirection => direction_signal(initial),
            Control::Direction => direction_signal(tangent),
        }
    }
}

fn direction_signal(degrees: f32) -> f32 {
    degrees.rem_euclid(360.0) / 360.0
}

/// One option set's stored state. Every block is off at its default, so a
/// default stroke paints exactly as it did before the dynamics existed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Dynamics {
    pub shape: ShapeDynamics,
    pub scatter: ScatterDynamics,
    pub color: ColorDynamics,
    pub transfer: TransferDynamics,
    pub pose: PoseDynamics,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeDynamics {
    pub enabled: bool,
    pub size_control: Control,
    pub min_diameter: u8,
    pub angle_control: Control,
    pub min_roundness: u8,
    pub roundness_control: Control,
    pub tilt_scale: u8,
    pub brush_projection: bool,
}

impl Default for ShapeDynamics {
    fn default() -> Self {
        Self {
            enabled: false,
            size_control: Control::Off,
            min_diameter: 0,
            angle_control: Control::Off,
            min_roundness: 0,
            roundness_control: Control::Off,
            tilt_scale: 100,
            brush_projection: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScatterDynamics {
    pub enabled: bool,
    pub control: Control,
    pub both_axes: bool,
    pub count_control: Control,
    pub count_jitter: u8,
}

impl Default for ScatterDynamics {
    fn default() -> Self {
        Self {
            // The pre-existing scatter always applied; the panel can turn it
            // off while retaining the values.
            enabled: true,
            control: Control::Off,
            // The pre-existing scatter spreads in both axes.
            both_axes: true,
            count_control: Control::Off,
            count_jitter: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorDynamics {
    pub enabled: bool,
    pub apply_per_tip: bool,
    pub fg_bg_jitter: u8,
    pub fg_bg_control: Control,
    pub hue_jitter: u8,
    pub saturation_jitter: u8,
    pub brightness_jitter: u8,
    pub purity: i8,
}

impl Default for ColorDynamics {
    fn default() -> Self {
        Self {
            enabled: false,
            apply_per_tip: false,
            fg_bg_jitter: 0,
            fg_bg_control: Control::Off,
            hue_jitter: 0,
            saturation_jitter: 0,
            brightness_jitter: 0,
            purity: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransferDynamics {
    pub enabled: bool,
    pub opacity_jitter: u8,
    pub opacity_control: Control,
    pub flow_jitter: u8,
    pub flow_control: Control,
}

impl Default for TransferDynamics {
    fn default() -> Self {
        Self {
            enabled: false,
            opacity_jitter: 0,
            opacity_control: Control::Off,
            flow_jitter: 0,
            flow_control: Control::Off,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseDynamics {
    pub enabled: bool,
    pub tilt_x: i16,
    pub tilt_y: i16,
    pub rotation: i16,
    pub pressure: u8,
    pub override_tilt_x: bool,
    pub override_tilt_y: bool,
    pub override_rotation: bool,
    pub override_pressure: bool,
}

impl Default for PoseDynamics {
    fn default() -> Self {
        Self {
            enabled: false,
            tilt_x: 0,
            tilt_y: 0,
            rotation: 0,
            pressure: 100,
            override_tilt_x: false,
            override_tilt_y: false,
            override_rotation: false,
            override_pressure: false,
        }
    }
}

impl Dynamics {
    /// True when no option set can change a dab. The GPU stroke declines a
    /// non-default block because its shader evaluates one constant tip.
    pub fn is_default(&self) -> bool {
        *self == Dynamics::default()
    }

    pub fn sanitized(mut self) -> Self {
        self.shape.min_diameter = self.shape.min_diameter.min(100);
        self.shape.min_roundness = self.shape.min_roundness.min(100);
        self.shape.tilt_scale = self.shape.tilt_scale.min(100);
        self.scatter.count_jitter = self.scatter.count_jitter.min(100);
        self.color.fg_bg_jitter = self.color.fg_bg_jitter.min(100);
        self.color.hue_jitter = self.color.hue_jitter.min(100);
        self.color.saturation_jitter = self.color.saturation_jitter.min(100);
        self.color.brightness_jitter = self.color.brightness_jitter.min(100);
        self.color.purity = self.color.purity.clamp(-100, 100);
        self.transfer.opacity_jitter = self.transfer.opacity_jitter.min(100);
        self.transfer.flow_jitter = self.transfer.flow_jitter.min(100);
        self.pose.tilt_x = self.pose.tilt_x.clamp(-90, 90);
        self.pose.tilt_y = self.pose.tilt_y.clamp(-90, 90);
        self.pose.rotation = self.pose.rotation.rem_euclid(360);
        self.pose.pressure = self.pose.pressure.min(100);
        self
    }

    /// The input after Brush Pose overrides have replaced the device values.
    pub fn effective_input(&self, input: DynamicsInput) -> DynamicsInput {
        let pose = self.pose;
        if !pose.enabled {
            return input;
        }
        DynamicsInput {
            pressure: if pose.override_pressure {
                pose.pressure as f32 / 100.0
            } else {
                input.pressure
            },
            tilt_x: if pose.override_tilt_x {
                pose.tilt_x as f32
            } else {
                input.tilt_x
            },
            tilt_y: if pose.override_tilt_y {
                pose.tilt_y as f32
            } else {
                input.tilt_y
            },
            rotation: if pose.override_rotation {
                pose.rotation as f32
            } else {
                input.rotation
            },
            wheel: input.wheel,
        }
    }

    pub fn size_factor(
        &self,
        jitter: u8,
        input: DynamicsInput,
        dab: u32,
        tangent: f32,
        initial: f32,
        rng: &mut u64,
    ) -> f32 {
        let shape = self.shape;
        let min = shape.min_diameter as f32 / 100.0;
        let base = if shape.enabled && shape.size_control != Control::Off {
            let signal = input.signal(shape.size_control, dab, tangent, initial);
            min + (1.0 - min) * signal
        } else {
            1.0
        };
        let jitter = jitter as f32 / 100.0;
        let value = if jitter > 0.0 {
            base * (1.0 - jitter * next_f32(rng))
        } else {
            base
        };
        value.clamp(min, 1.0)
    }

    /// The dab's roundness percentage. The old jitter subtracts percentage
    /// points; a Control scales the base between Minimum Roundness and the
    /// brush's roundness.
    #[allow(clippy::too_many_arguments)]
    pub fn roundness_percent(
        &self,
        base: u8,
        jitter: u8,
        input: DynamicsInput,
        dab: u32,
        tangent: f32,
        initial: f32,
        rng: &mut u64,
    ) -> u8 {
        let shape = self.shape;
        let min = shape.min_roundness as f32;
        let mut value = base as f32;
        if shape.enabled && shape.roundness_control != Control::Off {
            let signal = input.signal(shape.roundness_control, dab, tangent, initial);
            value = min + (base as f32 - min) * signal;
        }
        if jitter > 0 {
            value -= jitter as f32 * next_f32(rng);
        }
        value.clamp(min.min(base as f32), 100.0) as u8
    }

    #[allow(clippy::too_many_arguments)]
    pub fn angle(
        &self,
        base: i32,
        jitter: u16,
        input: DynamicsInput,
        dab: u32,
        tangent: f32,
        initial: f32,
        rng: &mut u64,
    ) -> i32 {
        let shape = self.shape;
        let mut angle = base as f32;
        if shape.enabled {
            match shape.angle_control {
                Control::Off => {}
                Control::Direction => angle = tangent,
                Control::InitialDirection => angle = initial,
                control => {
                    let signal = input.signal(control, dab, tangent, initial);
                    angle = base as f32 + (signal * 2.0 - 1.0) * 180.0;
                }
            }
        }
        if jitter > 0 {
            let turn = (next_f32(rng) * 2.0 - 1.0) * jitter as f32;
            angle += turn;
        }
        angle.round() as i32
    }

    /// Scatter offsets in units of `scatter% × diameter`, for a dab at
    /// `tangent` degrees.
    pub fn scatter_offset(
        &self,
        scatter: u16,
        input: DynamicsInput,
        dab: u32,
        tangent: f32,
        rng: &mut u64,
    ) -> (f32, f32) {
        let block = self.scatter;
        if !block.enabled || scatter == 0 {
            return (0.0, 0.0);
        }
        let signal = input.signal(block.control, dab, tangent, 0.0);
        if block.both_axes {
            let x = next_f32(rng) * 2.0 - 1.0;
            let y = next_f32(rng) * 2.0 - 1.0;
            (x * signal, y * signal)
        } else {
            let normal = (tangent + 90.0).to_radians();
            let reach = next_f32(rng) * 2.0 - 1.0;
            (normal.cos() * reach * signal, normal.sin() * reach * signal)
        }
    }

    pub fn count(&self, base: u8, input: DynamicsInput, dab: u32, rng: &mut u64) -> u8 {
        let block = self.scatter;
        if !block.enabled {
            return base.clamp(1, 16);
        }
        let mut count = base as f32;
        if block.count_control != Control::Off {
            let signal = input.signal(block.count_control, dab, 0.0, 0.0);
            count *= signal;
        }
        if block.count_jitter > 0 {
            let jitter = block.count_jitter as f32 / 100.0;
            count *= 1.0 + (next_f32(rng) * 2.0 - 1.0) * jitter;
        }
        count.round().clamp(1.0, 16.0) as u8
    }

    /// The per-dab opacity and flow factors, each capped at 1.0.
    pub fn transfer(&self, input: DynamicsInput, dab: u32, rng: &mut u64) -> (f32, f32) {
        let block = self.transfer;
        if !block.enabled {
            return (1.0, 1.0);
        }
        let opacity = transfer_factor(block.opacity_jitter, block.opacity_control, input, dab, rng);
        let flow = transfer_factor(block.flow_jitter, block.flow_control, input, dab, rng);
        (opacity, flow)
    }

    pub fn color(
        &self,
        foreground: Rgba,
        background: Rgba,
        input: DynamicsInput,
        dab: u32,
        rng: &mut u64,
    ) -> Rgba {
        let block = self.color;
        if !block.enabled {
            return foreground;
        }
        let mix = if block.fg_bg_control != Control::Off {
            input.signal(block.fg_bg_control, dab, 0.0, 0.0) * block.fg_bg_jitter as f32 / 100.0
        } else if block.fg_bg_jitter > 0 {
            block.fg_bg_jitter as f32 / 100.0 * next_f32(rng)
        } else {
            0.0
        };
        let out = lerp_rgba(foreground, background, mix.clamp(0.0, 1.0));
        let (mut h, mut s, mut v) = rgb_to_hsv(out);
        if block.hue_jitter > 0 {
            h = (h + (next_f32(rng) * 2.0 - 1.0) * block.hue_jitter as f32 / 100.0 * 360.0)
                .rem_euclid(360.0);
        }
        if block.saturation_jitter > 0 {
            s = (s + (next_f32(rng) * 2.0 - 1.0) * block.saturation_jitter as f32 / 100.0)
                .clamp(0.0, 1.0);
        }
        if block.brightness_jitter > 0 {
            v = (v + (next_f32(rng) * 2.0 - 1.0) * block.brightness_jitter as f32 / 100.0)
                .clamp(0.0, 1.0);
        }
        let purity = block.purity as f32 / 100.0;
        s = if purity < 0.0 {
            s * (1.0 + purity)
        } else {
            (s + (1.0 - s) * purity).clamp(0.0, 1.0)
        };
        let (r, g, b) = hsv_to_rgb(h, s, v);
        Rgba { r, g, b, a: out.a }
    }

    /// Brush Projection: tilt turns and flattens the tip.
    pub fn projected_tip(&self, angle: i32, roundness: u8, input: DynamicsInput) -> (i32, u8) {
        if !self.shape.enabled || !self.shape.brush_projection {
            return (angle, roundness);
        }
        let tilt = input.tilt_x.clamp(-90.0, 90.0);
        let scale = self.shape.tilt_scale as f32 / 100.0;
        let turned = angle as f32 + tilt * scale;
        let flatten = (tilt.abs() / 90.0 * 0.8).clamp(0.0, 0.8);
        let round = (roundness as f32 * (1.0 - flatten)).max(5.0);
        (turned.round() as i32, round as u8)
    }
}

fn transfer_factor(
    jitter: u8,
    control: Control,
    input: DynamicsInput,
    dab: u32,
    rng: &mut u64,
) -> f32 {
    let mut value = if control != Control::Off {
        input.signal(control, dab, 0.0, 0.0)
    } else {
        1.0
    };
    if jitter > 0 {
        value *= 1.0 - jitter as f32 / 100.0 * next_f32(rng);
    }
    value.clamp(0.0, 1.0)
}

fn lerp_rgba(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Rgba {
        r: mix(a.r, b.r),
        g: mix(a.g, b.g),
        b: mix(a.b, b.b),
        a: mix(a.a, b.a),
    }
}

fn rgb_to_hsv(c: Rgba) -> (f32, f32, f32) {
    let (r, g, b) = (c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let s = if max == 0.0 { 0.0 } else { delta / max };
    (h, s, max)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h.rem_euclid(360.0) {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let to = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (to(r), to(g), to(b))
}

pub(crate) fn next_f32(rng: &mut u64) -> f32 {
    *rng = rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *rng;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / 16_777_216.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StrokeConfig;
    use pictura_core::{BitDepth, Channel, ColorMode, Document, Layer, LayerStack, PsdRect};

    fn config() -> StrokeConfig {
        StrokeConfig::default()
    }

    fn input(pressure: f32) -> DynamicsInput {
        DynamicsInput::from_pressure(pressure)
    }

    fn shape_on(d: &mut Dynamics) -> &mut ShapeDynamics {
        d.shape.enabled = true;
        &mut d.shape
    }

    fn layer_doc(w: u32, h: u32) -> Document {
        let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
        let n = (w * h) as usize;
        let _ = doc.layers.push(Layer {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: h as i32,
                right: w as i32,
            },
            channels: [0, 1, 2, -1]
                .map(|id| Channel {
                    id,
                    data: vec![0; n].into(),
                })
                .into(),
            children: LayerStack::default(),
            ..Default::default()
        });
        doc
    }

    #[test]
    fn fade_reaches_the_minimum_and_stays() {
        let mut d = Dynamics::default();
        let shape = shape_on(&mut d);
        shape.size_control = Control::Fade(4);
        shape.min_diameter = 20;
        let mut rng = 1;
        let factors: Vec<f32> = (0..8)
            .map(|dab| d.size_factor(0, input(1.0), dab, 0.0, 0.0, &mut rng))
            .collect();
        assert_eq!(factors[0], 1.0);
        assert!(factors[2] < factors[0]);
        assert_eq!(factors[4], 0.2);
        assert_eq!(factors[7], 0.2);
    }

    #[test]
    fn pressure_raises_the_controlled_value() {
        let mut d = Dynamics::default();
        let shape = shape_on(&mut d);
        shape.size_control = Control::PenPressure;
        shape.min_diameter = 20;
        let mut rng = 1;
        let low = d.size_factor(0, input(0.25), 0, 0.0, 0.0, &mut rng);
        let high = d.size_factor(0, input(0.75), 0, 0.0, 0.0, &mut rng);
        assert!(high > low);
        assert!(high <= 1.0);
    }

    #[test]
    fn default_dynamics_paint_the_same_pixels() {
        let cfg = config();
        let mut a = layer_doc(40, 40);
        let mut b = a.clone();
        let samples = [
            crate::StrokeSample {
                x: 8.0,
                y: 8.0,
                pressure: 1.0,
            },
            crate::StrokeSample {
                x: 32.0,
                y: 20.0,
                pressure: 1.0,
            },
        ];
        crate::paint_stroke(&mut a, "0", &cfg, &samples);
        let mut with = cfg;
        with.dynamics = Dynamics::default();
        crate::paint_stroke(&mut b, "0", &with, &samples);
        assert_eq!(
            a.layers[0].channels[0].data.as_slice(),
            b.layers[0].channels[0].data.as_slice()
        );
    }

    #[test]
    fn direction_angle_follows_the_tangent() {
        let mut d = Dynamics::default();
        shape_on(&mut d).angle_control = Control::Direction;
        let mut rng = 1;
        assert_eq!(d.angle(0, 0, input(1.0), 0, 90.0, 0.0, &mut rng), 90);
        assert_eq!(d.angle(0, 0, input(1.0), 1, 180.0, 0.0, &mut rng), 180);
    }

    #[test]
    fn minimum_diameter_floors_size_jitter() {
        let mut d = Dynamics::default();
        shape_on(&mut d).min_diameter = 40;
        let mut rng = 7;
        for _ in 0..200 {
            assert!(d.size_factor(100, input(1.0), 0, 0.0, 0.0, &mut rng) >= 0.4);
        }
    }

    #[test]
    fn projection_turns_and_flattens_with_tilt() {
        let mut d = Dynamics::default();
        let shape = shape_on(&mut d);
        shape.brush_projection = true;
        let tilt = DynamicsInput {
            tilt_x: 45.0,
            ..DynamicsInput::default()
        };
        let (angle, roundness) = d.projected_tip(0, 100, tilt);
        assert_ne!(angle, 0);
        assert!(roundness < 100);
    }

    #[test]
    fn both_axes_selects_the_scatter_plane() {
        let mut d = Dynamics::default();
        d.scatter.enabled = true;
        let mut rng = 3;
        let mut both = (0.0f32, 0.0f32);
        for _ in 0..20 {
            let (x, y) = d.scatter_offset(100, input(1.0), 0, 0.0, &mut rng);
            both = (both.0 + x.abs(), both.1 + y.abs());
        }
        d.scatter.both_axes = false;
        let mut across = 0.0f32;
        let mut along = 0.0f32;
        for _ in 0..20 {
            let (x, y) = d.scatter_offset(100, input(1.0), 0, 0.0, &mut rng);
            across += x.abs();
            along += y.abs();
        }
        assert!(both.0 > 0.0 && both.1 > 0.0);
        assert!(across < 1.0e-3 && along > 0.0);
    }

    #[test]
    fn count_jitter_stays_in_range() {
        let mut d = Dynamics::default();
        d.scatter.enabled = true;
        d.scatter.count_jitter = 100;
        let mut rng = 11;
        for _ in 0..200 {
            let count = d.count(1, input(1.0), 0, &mut rng);
            assert!((1..=16).contains(&count));
        }
    }

    #[test]
    fn pressure_lowers_transfer() {
        let mut d = Dynamics::default();
        d.transfer.enabled = true;
        d.transfer.opacity_jitter = 100;
        d.transfer.opacity_control = Control::PenPressure;
        let mut rng = 1;
        let (opacity, _) = d.transfer(input(0.5), 0, &mut rng);
        assert!(opacity <= 0.5);
    }

    #[test]
    fn zero_transfer_jitter_is_a_no_op() {
        let mut d = Dynamics::default();
        d.transfer.enabled = true;
        let mut rng = 1;
        assert_eq!(d.transfer(input(0.3), 0, &mut rng), (1.0, 1.0));
    }

    fn painted_colors(doc: &Document) -> Vec<(u8, u8, u8)> {
        let r = doc.layers[0].channels[0].data.as_slice();
        let g = doc.layers[0].channels[1].data.as_slice();
        let b = doc.layers[0].channels[2].data.as_slice();
        let a = doc.layers[0].channels[3].data.as_slice();
        (0..r.len())
            .filter(|&i| a[i] > 0)
            .map(|i| (r[i], g[i], b[i]))
            .collect()
    }

    #[test]
    fn per_stroke_color_is_constant() {
        let mut doc = layer_doc(64, 64);
        let mut d = Dynamics::default();
        d.color.enabled = true;
        d.color.fg_bg_jitter = 100;
        let cfg = StrokeConfig {
            color: Rgba {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
            background: Rgba {
                r: 0,
                g: 0,
                b: 255,
                a: 255,
            },
            diameter: 8,
            spacing: crate::spacing::SpacingMode::Fixed(10),
            dynamics: d,
            ..StrokeConfig::default()
        };
        let samples = [
            crate::StrokeSample {
                x: 8.0,
                y: 8.0,
                pressure: 1.0,
            },
            crate::StrokeSample {
                x: 56.0,
                y: 32.0,
                pressure: 1.0,
            },
        ];
        crate::paint_stroke(&mut doc, "0", &cfg, &samples);
        let colors = painted_colors(&doc);
        assert!(!colors.is_empty());
        assert!(colors.iter().all(|c| *c == colors[0]));
    }

    #[test]
    fn per_tip_color_varies() {
        let mut doc = layer_doc(64, 64);
        let mut d = Dynamics::default();
        d.color.enabled = true;
        d.color.apply_per_tip = true;
        d.color.hue_jitter = 100;
        let cfg = StrokeConfig {
            color: Rgba {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
            diameter: 6,
            spacing: crate::spacing::SpacingMode::Fixed(200),
            dynamics: d,
            ..StrokeConfig::default()
        };
        let samples = [
            crate::StrokeSample {
                x: 8.0,
                y: 8.0,
                pressure: 1.0,
            },
            crate::StrokeSample {
                x: 56.0,
                y: 8.0,
                pressure: 1.0,
            },
        ];
        crate::paint_stroke(&mut doc, "0", &cfg, &samples);
        let colors = painted_colors(&doc);
        assert!(colors.len() > 1);
        assert!(colors.iter().any(|c| *c != colors[0]));
    }

    #[test]
    fn purity_desaturates() {
        let mut d = Dynamics::default();
        d.color.enabled = true;
        d.color.purity = -100;
        let fg = Rgba {
            r: 200,
            g: 30,
            b: 90,
            a: 255,
        };
        let mut rng = 1;
        let out = d.color(fg, fg, input(1.0), 0, &mut rng);
        assert_eq!((out.r, out.g, out.b), (out.r, out.r, out.r));
    }

    #[test]
    fn pose_override_replaces_the_device_input() {
        let mut d = Dynamics::default();
        d.pose.enabled = true;
        d.pose.override_pressure = true;
        d.pose.pressure = 50;
        let effective = d.effective_input(input(0.1));
        assert_eq!(effective.pressure, 0.5);
    }

    #[test]
    fn tilt_and_rotation_reach_the_signals() {
        let input = DynamicsInput {
            tilt_x: 45.0,
            rotation: 180.0,
            ..DynamicsInput::default()
        };
        assert!((input.signal(Control::PenTilt, 0, 0.0, 0.0) - 0.5).abs() < 1.0e-6);
        assert_eq!(input.signal(Control::Rotation, 0, 0.0, 0.0), 0.5);
    }
}
