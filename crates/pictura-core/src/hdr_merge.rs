//! Deterministic exposure-bracket reconstruction in linear RGB.

mod registration;

#[derive(Clone, Copy)]
pub struct Exposure<'a> {
    pub rgba: &'a [[u8; 4]],
    pub ev100: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResponseCurve([f64; 3]);

impl ResponseCurve {
    pub fn new(exponents: [f64; 3]) -> Option<Self> {
        exponents
            .iter()
            .all(|value| (0.5..=2.0).contains(value))
            .then_some(Self(exponents))
    }

    pub fn exponents(self) -> [f64; 3] {
        self.0
    }

    fn tables(self) -> [[f64; 256]; 3] {
        self.0.map(|exponent| {
            std::array::from_fn(|value| linearize(value as u8).powf(exponent))
        })
    }
}

#[derive(Default)]
pub struct MergeOptions {
    pub align: bool,
    pub ghost_base: Option<usize>,
    pub response: Option<ResponseCurve>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MergeError {
    InvalidBracket,
    InvalidExposure,
    Alignment(usize),
}

pub struct HdrMerge {
    pub planes: [Vec<f32>; 4],
    pub response: ResponseCurve,
}

struct Source<'a> {
    rgba: &'a [[u8; 4]],
    scale: f64,
    offset: (isize, isize),
}

impl Source<'_> {
    fn pixel(&self, x: usize, y: usize, width: usize, height: usize) -> Option<[u8; 4]> {
        let x = x.checked_add_signed(self.offset.0)?;
        let y = y.checked_add_signed(self.offset.1)?;
        if x >= width || y >= height {
            return None;
        }
        let px = self.rgba[y * width + x];
        (px[3] != 0).then_some(px)
    }
}

/// Peak working estimate callers add to their existing decoded-file allocations.
pub fn merge_working_bytes(width: u32, height: u32, sources: usize) -> Option<u64> {
    u64::from(width)
        .checked_mul(u64::from(height))?
        .checked_mul(96u64.checked_add(4u64.checked_mul(u64::try_from(sources).ok()?)?)?)
}

pub fn valid_bracket_ev(values: &[f64]) -> bool {
    if values.len() < 3 || values.iter().any(|v| !v.is_finite()) {
        return false;
    }
    let (lo, hi) = values.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
        (lo.min(v), hi.max(v))
    });
    (0.01..=32.0).contains(&(hi - lo))
}

/// Median exposure is independent of the chosen ghost-removal base.
pub fn median_exposure(values: &[f64]) -> Option<usize> {
    if !valid_bracket_ev(values) {
        return None;
    }
    let mut order: Vec<_> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
    Some(order[order.len() / 2])
}

pub fn merge(
    exposures: &[Exposure<'_>],
    width: u32,
    height: u32,
    options: &MergeOptions,
) -> Result<HdrMerge, MergeError> {
    let (width, height) = (width as usize, height as usize);
    let size = width
        .checked_mul(height)
        .filter(|&size| size > 0)
        .ok_or(MergeError::InvalidBracket)?;
    if exposures.len() < 3
        || exposures.iter().any(|input| input.rgba.len() != size)
        || options.ghost_base.is_some_and(|base| base >= exposures.len())
    {
        return Err(MergeError::InvalidBracket);
    }

    let values: Vec<_> = exposures.iter().map(|input| input.ev100).collect();
    let median = median_exposure(&values).ok_or(MergeError::InvalidExposure)?;
    let mut sources = Vec::with_capacity(exposures.len());
    for (index, exposure) in exposures.iter().enumerate() {
        let offset = if options.align && index != median {
            registration::offset(exposures[median].rgba, exposure.rgba, width, height)
                .ok_or(MergeError::Alignment(index))?
        } else {
            (0, 0)
        };
        sources.push(Source {
            rgba: exposure.rgba,
            scale: (values[median] - exposure.ev100).exp2(),
            offset,
        });
    }

    let response = options
        .response
        .unwrap_or_else(|| estimate_response(&sources, median, width, height));
    let tables = response.tables();
    let mut planes = std::array::from_fn(|_| Vec::with_capacity(size));
    for y in 0..height {
        for x in 0..width {
            let pixel = fuse_pixel(
                &sources,
                &tables,
                (x, y),
                (width, height),
                median,
                options.ghost_base,
            );
            for (plane, value) in planes.iter_mut().zip(pixel) {
                plane.push(value);
            }
        }
    }
    Ok(HdrMerge { planes, response })
}

fn linearize(code: u8) -> f64 {
    let value = f64::from(code) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn estimate_response(
    sources: &[Source<'_>],
    anchor: usize,
    width: usize,
    height: usize,
) -> ResponseCurve {
    let logs: [f64; 256] = std::array::from_fn(|i| linearize(i as u8).ln());
    let mut estimates: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::new());
    let stride = (width * height).div_ceil(1024).max(1);
    for i in (0..width * height).step_by(stride) {
        let (x, y) = (i % width, i / width);
        let Some(base) = sources[anchor].pixel(x, y, width, height) else {
            continue;
        };
        for source in sources {
            let stops = source.scale.ln();
            if stops.abs() < 0.2 {
                continue;
            }
            let Some(pixel) = source.pixel(x, y, width, height) else {
                continue;
            };
            for (channel, estimates) in estimates.iter_mut().enumerate() {
                let (a, b) = (base[channel], pixel[channel]);
                if !(20..=235).contains(&a) || !(20..=235).contains(&b) {
                    continue;
                }
                let ratio = logs[b as usize] - logs[a as usize];
                let exponent = stops / ratio;
                if (0.5..=2.0).contains(&exponent) {
                    estimates.push(exponent);
                }
            }
        }
    }
    ResponseCurve(estimates.map(|mut channel| {
        if channel.len() < 16 {
            return 1.0;
        }
        let middle = channel.len() / 2;
        *channel.select_nth_unstable_by(middle, f64::total_cmp).1
    }))
}

#[derive(Clone, Copy)]
struct Sample {
    rgba: [f64; 4],
    scale: f64,
    weights: [f64; 3],
    luma: f64,
    code: [u8; 4],
}

fn sample(code: [u8; 4], scale: f64, tables: &[[f64; 256]; 3]) -> Sample {
    let alpha = f64::from(code[3]) / 255.0;
    let rgb: [f64; 3] = std::array::from_fn(|c| tables[c][code[c] as usize] / scale);
    Sample {
        rgba: [rgb[0], rgb[1], rgb[2], alpha],
        scale,
        weights: std::array::from_fn(|c| f64::from(code[c].min(255 - code[c])) * alpha),
        luma: 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2],
        code,
    }
}

fn fuse_pixel(
    sources: &[Source<'_>],
    tables: &[[f64; 256]; 3],
    (x, y): (usize, usize),
    (width, height): (usize, usize),
    median: usize,
    ghost_base: Option<usize>,
) -> [f32; 4] {
    let base = ghost_base.unwrap_or(median);
    let mut base_sample = None;
    let mut nearest = None::<Sample>;
    let mut darkest = None::<Sample>;
    let mut brightest = None::<Sample>;
    let mut all_highlights = [true; 3];
    let mut all_shadows = [true; 3];
    let mut weights = [0.0; 3];
    let mut sums = [0.0; 3];
    let mut alpha_sum = 0.0;
    let mut alpha_weight = 0.0;
    let mut min_luma = f64::INFINITY;
    let mut max_luma = 0.0f64;
    for (index, source) in sources.iter().enumerate() {
        let Some(code) = source.pixel(x, y, width, height) else { continue };
        let current = sample(code, source.scale, tables);
        if index == base { base_sample = Some(current); }
        let distance = (current.scale / sources[base].scale).log2().abs();
        if nearest.is_none_or(|previous| distance < (previous.scale / sources[base].scale).log2().abs()) {
            nearest = Some(current);
        }
        if darkest.is_none_or(|previous| current.scale < previous.scale) { darkest = Some(current); }
        if brightest.is_none_or(|previous| current.scale > previous.scale) { brightest = Some(current); }
        for c in 0..3 {
            all_highlights[c] &= code[c] == 255;
            all_shadows[c] &= code[c] == 0;
            weights[c] += current.weights[c];
            sums[c] += current.weights[c] * current.rgba[c];
        }
        let w = current.weights.iter().copied().fold(0.0_f64, f64::max);
        if w > 0.0 {
            alpha_sum += w * current.rgba[3];
            alpha_weight += w;
            min_luma = min_luma.min(current.luma);
            max_luma = max_luma.max(current.luma);
        }
    }
    let Some(nearest) = nearest else { return [0.0; 4] };
    let ghost = ghost_base.is_some() && max_luma > 1.35 * min_luma + 0.02;
    let mut output = [0.0_f32; 4];
    for c in 0..3 {
        let fallback = if all_highlights[c] { darkest } else if all_shadows[c] { brightest } else { Some(nearest) };
        output[c] = if ghost && base_sample.is_some_and(|sample| sample.code[c] > 0 && sample.code[c] < 255) {
            base_sample.unwrap().rgba[c] as f32
        } else if weights[c] > 0.0 {
            (sums[c] / weights[c]) as f32
        } else {
            fallback.unwrap_or(nearest).rgba[c] as f32
        };
    }
    output[3] = if ghost && base_sample.is_some_and(|sample| sample.weights.iter().any(|&w| w > 0.0)) {
        base_sample.unwrap().rgba[3] as f32
    } else if alpha_weight > 0.0 {
        (alpha_sum / alpha_weight) as f32
    } else {
        nearest.rgba[3] as f32
    };
    output
}

#[cfg(test)]
mod tests;
