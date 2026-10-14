//! Lighting Effects (`FILT-103`): CS6's light rig — up to sixteen Spot, Point,
//! and Infinite lights over one set of surface properties — re-lighting the
//! picture as a surface.
//!
//! The shading is photorust's (perfecto25/photorust `core/src/filters/render.rs`,
//! GPL-3.0): ambient, diffuse by `N·L`, and a Blinn highlight, with the normal
//! read off whichever channel the Texture list names. Kooka widens its single
//! lamp to the rig and gives each light CS6's on-canvas geometry: a Spot is an
//! ellipse with a hotspot ellipse inside it, tangent at the far end; a Point is
//! a circle; an Infinite light is a direction and an elevation. Positions are
//! fractions of the frame and sizes fractions of the half-diagonal, so a light
//! on the workspace's proxy and on the full image are the same picture.
//!
//! ponytail: CS6's lighting model is closed; this is tuned by eye against CS6
//! screenshots, not fitted to reference output.

use pictura_core::PixelBuffer;
use rayon::prelude::*;

use crate::{validate, FilterError};

/// CS6's cap on lights in one rig.
pub const MAX_LIGHTS: usize = 16;

/// The three lights CS6's Lighting Effects offers, in the order its light type
/// list gives them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LightType {
    #[default]
    Spot,
    Point,
    Infinite,
}

/// Which channel is read as a height map — CS6's Texture list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextureChannel {
    #[default]
    None,
    Red,
    Green,
    Blue,
}

/// One light: the upper half of CS6's Properties panel plus the handles
/// dragged on the canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub kind: LightType,
    /// The Lights panel's eye: a hidden light contributes nothing.
    pub on: bool,
    pub color: [u8; 3],
    /// -100..=100; about 50 leaves the picture as bright as it found it, and a
    /// negative intensity takes light away.
    pub intensity: f32,
    /// -100..=100: how much of a Spot's ellipse is at full brightness.
    pub hotspot: f32,
    /// The Spot's ellipse centre, the Point's position, or where the
    /// Infinite light's widget sits; fractions of the frame.
    pub center: (f32, f32),
    /// Degrees, screen-oriented (y down): the way a Spot aims (centre to the
    /// hotspot end) or the way an Infinite light comes from.
    pub angle: f32,
    /// A Spot's semi-major axis or a Point's radius, as a fraction of the
    /// half-diagonal.
    pub size: f32,
    /// A Spot's semi-minor axis, as a fraction of the half-diagonal.
    pub width: f32,
    /// Degrees above the picture an Infinite light shines from, 0..=90.
    pub elevation: f32,
}

impl Default for Light {
    /// CS6's Default style: a white spotlight, intensity 35, focus 69.
    fn default() -> Self {
        Self {
            kind: LightType::Spot,
            on: true,
            color: [255, 255, 255],
            intensity: 35.0,
            hotspot: 69.0,
            center: (0.5, 0.5),
            angle: 325.0,
            size: 0.6,
            width: 0.32,
            elevation: 50.0,
        }
    }
}

/// The whole rig: every light plus the lower half of CS6's Properties panel,
/// which applies to all of them.
#[derive(Debug, Clone, PartialEq)]
pub struct Lighting {
    /// The colour of the light that is there without any lamp, and how much of
    /// it (-100..=100).
    pub colorize: [u8; 3],
    pub ambience: f32,
    /// A stop control over the whole result (-100..=100).
    pub exposure: f32,
    /// How sharp the highlight is (-100 matte to 100 shiny) and whose colour
    /// it takes (-100 plastic, the lamp's, to 100 metallic, the surface's).
    pub gloss: f32,
    pub metallic: f32,
    /// The channel read as a height map, and how tall it stands (0..=100).
    pub texture: TextureChannel,
    pub height: f32,
    /// 1..=[`MAX_LIGHTS`] lights.
    pub lights: Vec<Light>,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            colorize: [255, 255, 255],
            ambience: 0.0,
            exposure: 0.0,
            gloss: 0.0,
            metallic: 0.0,
            texture: TextureChannel::None,
            height: 50.0,
            lights: vec![Light::default()],
        }
    }
}

/// A Spot's hotspot ellipse in units of its outer ellipse: the semi-axes as
/// fractions of the outer semi-axes, and how far its centre sits from the
/// outer centre along the aim, as a fraction of the semi-major axis. It sits
/// toward the far end, just short of touching it so the light there fades
/// rather than cutting off. The workspace draws the same
/// ellipse (`lighting_rig.cpp`), so the two must change together.
pub fn spot_hotspot(hotspot: f32) -> (f32, f32, f32) {
    let h = ((hotspot.clamp(-100.0, 100.0) + 100.0) / 200.0).clamp(0.0, 1.0);
    let major = 0.6 * h * h;
    let minor = 0.7 * h;
    (major, minor, 0.9 * (1.0 - major))
}

fn dot3(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1 + a.2 * b.2
}

fn normalize3(v: (f32, f32, f32)) -> (f32, f32, f32) {
    let length = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
    if length <= f32::EPSILON {
        return (0.0, 0.0, 1.0);
    }
    (v.0 / length, v.1 / length, v.2 / length)
}

/// One light resolved to pixels for the frame being lit.
struct Lamp {
    kind: LightType,
    color: (f32, f32, f32),
    strength: f32,
    at: (f32, f32),
    dir: (f32, f32),
    /// Spot: outer semi-axes; Point: radius twice.
    axes: (f32, f32),
    /// Spot: hotspot semi-axes and centre offset along `dir`.
    inner: (f32, f32, f32),
    /// Where the light hangs above the picture (Spot, Point), or the unit
    /// direction it comes from (Infinite).
    source: (f32, f32, f32),
}

impl Lamp {
    fn new(light: &Light, w: f32, h: f32, span: f32) -> Lamp {
        let heading = light.angle.to_radians();
        let dir = (heading.cos(), heading.sin());
        let at = (light.center.0 * w, light.center.1 * h);
        let major = light.size.clamp(0.01, 3.0) * span;
        let minor = light.width.clamp(0.01, 3.0) * span;
        let (inner_a, inner_b, offset) = spot_hotspot(light.hotspot);
        let source = match light.kind {
            LightType::Spot => (at.0, at.1, major * 0.8),
            LightType::Point => (at.0, at.1, major * 0.7),
            LightType::Infinite => {
                let elevation = light.elevation.clamp(0.0, 90.0).to_radians();
                let flat = elevation.cos();
                normalize3((dir.0 * flat, dir.1 * flat, elevation.sin()))
            }
        };
        Lamp {
            kind: light.kind,
            color: (
                light.color[0] as f32 / 255.0,
                light.color[1] as f32 / 255.0,
                light.color[2] as f32 / 255.0,
            ),
            strength: light.intensity.clamp(-100.0, 100.0) / 50.0,
            at,
            dir,
            axes: match light.kind {
                LightType::Spot => (major, minor),
                _ => (major, major),
            },
            inner: (inner_a * major, inner_b * minor, offset * major),
            source,
        }
    }

    /// How much of the light reaches `p`, 0..=1, before shading.
    fn falloff(&self, p: (f32, f32)) -> f32 {
        let (dx, dy) = (p.0 - self.at.0, p.1 - self.at.1);
        match self.kind {
            LightType::Infinite => 1.0,
            LightType::Point => {
                let away = dx.hypot(dy) / self.axes.0;
                (1.0 - away * away).max(0.0)
            }
            LightType::Spot => {
                if dx.hypot(dy) > self.axes.0.max(self.axes.1) {
                    return 0.0;
                }
                // Normalised distances to the outer and hotspot ellipses (1 on
                // the edge); between the two, the light fades by how far across
                // the band `p` lies.
                let outer = self.spot_distance((dx, dy), (self.axes.0, self.axes.1, 0.0));
                if outer >= 1.0 {
                    return 0.0;
                }
                let inner = self.spot_distance((dx, dy), self.inner);
                if inner <= 1.0 {
                    return 1.0;
                }
                let t = 1.0 - (inner - 1.0) / (inner - outer);
                t * t * (3.0 - 2.0 * t)
            }
        }
    }

    /// The normalised distance of `d` (offset from the outer centre) from the
    /// ellipse with semi-axes `(a, b)` centred `shift` along the aim.
    fn spot_distance(&self, d: (f32, f32), (a, b, shift): (f32, f32, f32)) -> f32 {
        let along = d.0 * self.dir.0 + d.1 * self.dir.1 - shift;
        let across = -d.0 * self.dir.1 + d.1 * self.dir.0;
        ((along / a.max(1e-4)).powi(2) + (across / b.max(1e-4)).powi(2)).sqrt()
    }

    /// The unit direction from `p` to the light.
    fn toward(&self, p: (f32, f32)) -> (f32, f32, f32) {
        match self.kind {
            LightType::Infinite => self.source,
            _ => normalize3((self.source.0 - p.0, self.source.1 - p.1, self.source.2)),
        }
    }
}

fn check(name: &str, value: f32, lo: f32, hi: f32) -> Result<(), FilterError> {
    if value.is_finite() && (lo..=hi).contains(&value) {
        Ok(())
    } else {
        Err(FilterError::InvalidParams(format!(
            "lighting {name} {value} out of range {lo}..={hi}"
        )))
    }
}

fn validate_rig(opt: &Lighting) -> Result<(), FilterError> {
    for (name, value) in [
        ("ambience", opt.ambience),
        ("exposure", opt.exposure),
        ("gloss", opt.gloss),
        ("metallic", opt.metallic),
    ] {
        check(name, value, -100.0, 100.0)?;
    }
    check("height", opt.height, 0.0, 100.0)?;
    if opt.lights.is_empty() || opt.lights.len() > MAX_LIGHTS {
        return Err(FilterError::InvalidParams(format!(
            "lighting needs 1..={MAX_LIGHTS} lights, got {}",
            opt.lights.len()
        )));
    }
    for light in &opt.lights {
        check("intensity", light.intensity, -100.0, 100.0)?;
        check("hotspot", light.hotspot, -100.0, 100.0)?;
        check("size", light.size, 0.001, 10.0)?;
        check("width", light.width, 0.001, 10.0)?;
        check("elevation", light.elevation, 0.0, 90.0)?;
        if !light.center.0.is_finite() || !light.center.1.is_finite() || !light.angle.is_finite() {
            return Err(FilterError::InvalidParams(
                "lighting center/angle must be finite".into(),
            ));
        }
    }
    Ok(())
}

/// Light the picture as though the rig were shining on it — Filter ▸ Render ▸
/// Lighting Effects.
///
/// Not light *added* the way a flare is: a pixel comes out as what it
/// reflects, so where no light reaches and there is no ambience it goes black
/// however bright it started. Every visible light adds its diffuse term (and,
/// with Gloss above zero, its highlight); Ambience, tinted by Colorize, lights
/// everything evenly; Exposure scales the result in stops. Alpha is untouched.
pub fn lighting_effects(buf: &mut PixelBuffer, opt: &Lighting) -> Result<(), FilterError> {
    let n = validate(buf)?;
    validate_rig(opt)?;

    let w = buf.width as usize;
    let h = buf.height as usize;
    let (fw, fh) = (w as f32, h as f32);
    let span = 0.5 * fw.hypot(fh);
    let lamps: Vec<Lamp> = opt
        .lights
        .iter()
        .filter(|light| light.on)
        .map(|light| Lamp::new(light, fw, fh, span))
        .collect();

    let ambient = opt.ambience / 100.0;
    let fill = (
        opt.colorize[0] as f32 / 255.0 * ambient,
        opt.colorize[1] as f32 / 255.0 * ambient,
        opt.colorize[2] as f32 / 255.0 * ambient,
    );
    // Exposure in stops: fifty points either way is twice or half the light.
    let exposure = (opt.exposure / 50.0).exp2();
    // A highlight only once Gloss is positive: at zero a flat surface faces
    // every lamp alike and a highlight would wash the picture white.
    let shine = (opt.gloss / 100.0).max(0.0).powf(1.5);
    let tightness = 2.0f32.powf(1.0 + (opt.gloss + 100.0) / 200.0 * 6.0);
    // Plastic reflects the lamp's colour, metal its own.
    let metal = ((opt.metallic + 100.0) / 200.0).clamp(0.0, 1.0);

    // The height map, read before anything is written: a normal needs the
    // neighbours of its pixel, and those are about to change.
    let relief = (opt.texture != TextureChannel::None).then(|| {
        let channel = match opt.texture {
            TextureChannel::Red => 0,
            TextureChannel::Green => 1,
            _ => 2,
        };
        buf.data[channel * n..channel * n + n]
            .iter()
            .map(|&v| v as f32 / 255.0)
            .collect::<Vec<f32>>()
    });
    let relief_scale = opt.height / 100.0 * span * 0.05;
    let normal_at = |x: usize, y: usize| match &relief {
        None => (0.0, 0.0, 1.0),
        Some(map) => {
            let sample = |sx: usize, sy: usize| map[sy * w + sx];
            let (left, right) = (x.saturating_sub(1), (x + 1).min(w - 1));
            let (up, down) = (y.saturating_sub(1), (y + 1).min(h - 1));
            normalize3((
                (sample(left, y) - sample(right, y)) * relief_scale,
                (sample(x, up) - sample(x, down)) * relief_scale,
                1.0,
            ))
        }
    };

    let (r, rest) = buf.data.split_at_mut(n);
    let (g, rest) = rest.split_at_mut(n);
    let b = &mut rest[..n];
    r.par_chunks_exact_mut(w)
        .zip(g.par_chunks_exact_mut(w))
        .zip(b.par_chunks_exact_mut(w))
        .enumerate()
        .for_each(|(y, ((r, g), b))| {
            let py = y as f32 + 0.5;
            for x in 0..w {
                let p = (x as f32 + 0.5, py);
                let normal = normal_at(x, y);
                let mut light = fill;
                let mut spec_lamp = (0.0f32, 0.0f32, 0.0f32);
                let mut spec_own = 0.0f32;
                for lamp in &lamps {
                    let falloff = lamp.falloff(p);
                    if falloff <= 0.0 {
                        continue;
                    }
                    let toward = lamp.toward(p);
                    // Relative to a flat pixel, so with no texture a lamp lights
                    // its footprint by its falloff alone; a slope facing it
                    // catches more, one facing away less.
                    let facing = (dot3(normal, toward).max(0.0) / toward.2.max(0.05)).min(2.0);
                    let diffuse = facing * falloff * lamp.strength;
                    light.0 += lamp.color.0 * diffuse;
                    light.1 += lamp.color.1 * diffuse;
                    light.2 += lamp.color.2 * diffuse;
                    if shine > 0.0 {
                        let half = normalize3((toward.0, toward.1, toward.2 + 1.0));
                        let highlight = dot3(normal, half).max(0.0).powf(tightness)
                            * falloff
                            * shine
                            * lamp.strength.abs();
                        spec_lamp.0 += highlight * lamp.color.0;
                        spec_lamp.1 += highlight * lamp.color.1;
                        spec_lamp.2 += highlight * lamp.color.2;
                        spec_own += highlight;
                    }
                }
                for (out, (lit, lamp_spec)) in [&mut r[x], &mut g[x], &mut b[x]].into_iter().zip([
                    (light.0, spec_lamp.0),
                    (light.1, spec_lamp.1),
                    (light.2, spec_lamp.2),
                ]) {
                    let own = *out as f32 / 255.0;
                    let spec = lamp_spec * (1.0 - metal) + spec_own * own * metal;
                    let value = (own * lit + spec) * exposure;
                    *out = (value * 255.0).clamp(0.0, 255.0) as u8;
                }
            }
        });
    Ok(())
}

#[cfg(test)]
#[path = "lighting_tests.rs"]
mod tests;
