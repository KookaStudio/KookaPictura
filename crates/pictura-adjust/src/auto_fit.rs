//! Configurable automatic correction. Histograms retain source precision; the
//! accumulator can also ingest bounded composite bands for an adjustment layer.

use pictura_core::{AutoKind, AutoOptions, LevelsChannel, PixelBuffer, Sample};

const IDENTITY: LevelsChannel = LevelsChannel {
    input_black: 0,
    input_white: 255,
    output_black: 0,
    output_white: 255,
    gamma: 1.0,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoTransfer {
    low: f64,
    high: f64,
    shadow: f64,
    highlight: f64,
    gamma: f64,
}

impl AutoTransfer {
    pub fn map(self, value: f64) -> f64 {
        let t = ((value - self.low) / (self.high - self.low)).clamp(0.0, 1.0);
        self.shadow + (self.highlight - self.shadow) * t.powf(1.0 / self.gamma)
    }

    pub fn levels(self) -> LevelsChannel {
        let lo = u8::from_unit(self.low);
        let hi = u8::from_unit(self.high);
        if hi <= lo {
            return IDENTITY;
        }
        LevelsChannel {
            input_black: lo,
            input_white: hi,
            output_black: u8::from_unit(self.shadow),
            output_white: u8::from_unit(self.highlight),
            gamma: self.gamma,
        }
    }

    pub fn curve(self) -> Vec<(u8, u8)> {
        let lo = u8::from_unit(self.low);
        let hi = u8::from_unit(self.high);
        if hi <= lo {
            return vec![(0, 0), (255, 255)];
        }
        let mut xs = vec![0, lo, hi, 255];
        if self.gamma != 1.0 {
            xs.extend((1..8).map(|i| lo + ((hi - lo) as u16 * i / 8) as u8));
        }
        xs.sort_unstable();
        xs.dedup();
        xs.into_iter()
            .map(|x| (x, u8::from_unit(self.map(x.to_unit()))))
            .collect()
    }
}

#[derive(Default)]
struct ColorBin {
    count: u64,
    sum: [f64; 3],
}

impl ColorBin {
    fn add(&mut self, sample: [f64; 3]) {
        self.count += 1;
        for (sum, value) in self.sum.iter_mut().zip(sample) {
            *sum += value;
        }
    }
}

pub struct AutoHistogram {
    channels: [Vec<u64>; 3],
    luma: Vec<ColorBin>,
    neutral: ColorBin,
    total: u64,
}

impl AutoHistogram {
    pub fn new(maximum: u16) -> Self {
        let bins = usize::from(maximum) + 1;
        Self {
            channels: std::array::from_fn(|_| vec![0; bins]),
            luma: (0..bins).map(|_| ColorBin::default()).collect(),
            neutral: ColorBin::default(),
            total: 0,
        }
    }

    pub fn add_planar(&mut self, image: &PixelBuffer) {
        let n = image.width as usize * image.height as usize;
        let alpha = image.data.get(3 * n..4 * n);
        let colors = &image.data[..3 * n];
        self.add_planes([&colors[..n], &colors[n..2 * n], &colors[2 * n..]], |i| {
            alpha.is_none_or(|a| a[i] != 0)
        });
    }

    pub fn add_planes<T: Sample>(&mut self, planes: [&[T]; 3], use_pixel: impl Fn(usize) -> bool) {
        let last = (self.luma.len() - 1) as f64;
        let bin = |v: f64| (v * last).round() as usize;
        for i in 0..planes[0].len() {
            if !use_pixel(i) {
                continue;
            }
            let px = planes.map(|plane| plane[i].to_unit());
            for (channel, value) in self.channels.iter_mut().zip(px) {
                channel[bin(value)] += 1;
            }
            let luma = crate::common::luma(px[0], px[1], px[2]);
            self.luma[bin(luma)].add(px);
            let spread = px.into_iter().fold(f64::NEG_INFINITY, f64::max)
                - px.into_iter().fold(f64::INFINITY, f64::min);
            if (0.25..=0.75).contains(&luma) && spread <= 0.20 {
                self.neutral.add(px);
            }
            self.total += 1;
        }
    }

    /// The bin that contains the first retained sample, traversing from an end.
    fn boundary(mut counts: impl Iterator<Item = u64>, cut: u64) -> usize {
        let mut seen = 0;
        counts
            .position(|count| {
                seen += count;
                seen > cut
            })
            .unwrap_or_default()
    }

    fn band_color<'a>(
        bins: impl Iterator<Item = &'a ColorBin>,
        mut count: u64,
    ) -> [f64; 3] {
        let total = count;
        let mut sum = [0.0; 3];
        for bin in bins {
            if count == 0 {
                break;
            }
            if bin.count == 0 {
                continue;
            }
            let take = count.min(bin.count);
            for (out, value) in sum.iter_mut().zip(bin.sum) {
                *out += value * (take as f64 / bin.count as f64);
            }
            count -= take;
        }
        sum.map(|value| value / total as f64)
    }

    pub fn fit(&self, options: &AutoOptions) -> [Option<AutoTransfer>; 3] {
        if self.total < 2 || self.channels.iter().all(|counts| {
            counts.iter().filter(|&&n| n != 0).take(2).count() <= 1
        }) {
            return [None; 3];
        }

        let last = self.luma.len() - 1;
        let cut = |total: u64, percent: f64| {
            (total as f64 * (percent.clamp(0.0, 49.99) / 100.0)).floor() as u64
        };
        let shadow_cut = cut(self.total, options.shadow_clip);
        let highlight_cut = cut(self.total, options.highlight_clip);
        let bounds = |hist: &[u64], dark, light| {
            let low = Self::boundary(hist.iter().copied(), dark);
            let high = last - Self::boundary(hist.iter().rev().copied(), light);
            (low as f64 / last as f64, high as f64 / last as f64)
        };
        let endpoints = match options.algorithm {
            AutoKind::BrightnessContrast => {
                let luminance: Vec<u64> = self.luma.iter().map(|bin| bin.count).collect();
                let fixed_cut = cut(self.total, 0.1);
                [bounds(&luminance, fixed_cut, fixed_cut); 3]
            }
            AutoKind::Tone => self.channels.each_ref().map(|counts| bounds(counts, shadow_cut, highlight_cut)),
            AutoKind::Contrast => {
                let shared: Vec<u64> = (0..=last)
                    .map(|i| self.channels.iter().map(|counts| counts[i]).sum())
                    .collect();
                [bounds(&shared, cut(self.total * 3, options.shadow_clip),
                    cut(self.total * 3, options.highlight_clip)); 3]
            }
            AutoKind::Color => {
                let low = Self::band_color(self.luma.iter(), shadow_cut + 1);
                let high = Self::band_color(self.luma.iter().rev(), highlight_cut + 1);
                std::array::from_fn(|c| (low[c], high[c]))
            }
        };

        std::array::from_fn(|c| {
            let (low, high) = endpoints[c];
            if low >= high {
                return None;
            }
            let brightness_contrast = options.algorithm == AutoKind::BrightnessContrast;
            let shadow = if brightness_contrast { 0.0 } else { options.shadow[c].to_unit() };
            let highlight = if brightness_contrast { 1.0 } else { options.highlight[c].to_unit() };
            let mut gamma = 1.0;
            if !brightness_contrast && options.snap_midtones && self.neutral.count != 0 && shadow != highlight {
                let mean = self.neutral.sum[c] / self.neutral.count as f64;
                let input = (mean - low) / (high - low);
                let output = (options.midtone[c].to_unit() - shadow) / (highlight - shadow);
                if input > 0.0 && input < 1.0 && output > 0.0 && output < 1.0 {
                    gamma = (input.ln() / output.ln()).clamp(0.1, 9.99);
                }
            }
            Some(AutoTransfer { low, high, shadow, highlight, gamma })
        })
    }
}

pub(crate) fn apply_configured<T: Sample>(
    options: &AutoOptions,
    data: &mut [T],
    n: usize,
    maximum: u16,
    include: impl Fn(usize) -> bool,
) {
    let (r, rest) = data.split_at_mut(n);
    let (g, rest) = rest.split_at_mut(n);
    let (b, _) = rest.split_at_mut(n);
    let mut histogram = AutoHistogram::new(maximum);
    histogram.add_planes([&*r, &*g, &*b], include);
    for (plane, transfer) in [r, g, b].into_iter().zip(histogram.fit(options)) {
        if let Some(transfer) = transfer {
            for value in plane {
                *value = T::from_unit(transfer.map(value.to_unit()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_auto_fits_algorithms_and_targets() {
        let r = [10u8, 50, 110, 190, 250];
        let g = [15u8, 55, 112, 185, 240];
        let b = [20u8, 60, 114, 180, 230];
        let mut histogram = AutoHistogram::new(255);
        assert_eq!(histogram.fit(&AutoOptions::default()), [None; 3]);
        histogram.add_planes([&r, &g, &b], |_| true);
        let mut options = AutoOptions {
            shadow_clip: 20.0, highlight_clip: 20.0, shadow: [20; 3],
            highlight: [235; 3], midtone: [145; 3], ..AutoOptions::default()
        };
        let tone = histogram.fit(&options);
        assert_eq!(tone[0].unwrap().levels().input_black, 50);
        assert_eq!(tone[2].unwrap().levels().input_white, 180);
        assert_eq!(u8::from_unit(tone[0].unwrap().map(0.0)), 20);
        options.algorithm = AutoKind::Contrast;
        let mono = histogram.fit(&options);
        assert_eq!(mono[0], mono[2]);
        options.algorithm = AutoKind::Color;
        let color = histogram.fit(&options);
        assert_ne!(color[0], tone[0]);
        options.snap_midtones = true;
        assert_ne!(histogram.fit(&options), color);

        let mut flat = AutoHistogram::new(255);
        flat.add_planes([&[15u8; 4], &[22u8; 4], &[44u8; 4]], |_| true);
        assert_eq!(flat.fit(&options), [None; 3]);
    }

    #[test]
    fn brightness_contrast_uses_luminance_and_ignores_target_settings() {
        let mut histogram = AutoHistogram::new(255);
        histogram.add_planes([&[20u8, 80, 180], &[40u8, 100, 200], &[60u8, 120, 220]], |_| true);
        let mut options = AutoOptions { algorithm: AutoKind::BrightnessContrast, ..AutoOptions::default() };
        let fit = histogram.fit(&options);
        assert_eq!(fit[0], fit[1]);
        assert_eq!(fit[1], fit[2]);
        assert!(fit[0].is_some());
        options.shadow = [50; 3];
        options.highlight = [180; 3];
        options.snap_midtones = true;
        options.shadow_clip = 30.0;
        assert_eq!(fit, histogram.fit(&options));
    }

    #[test]
    fn configured_auto_keeps_u16_precision() {
        let input = [1200u16, 1223, 1300];
        let mut data = [input, input, input, [17, 18, 19]].concat();
        apply_configured(&AutoOptions { shadow_clip: 0.0, highlight_clip: 0.0,
            ..AutoOptions::default() }, &mut data, 3, u16::MAX, |_| true);
        assert_eq!(&data[..3], &[0, 15073, 65535]);
        assert_eq!(&data[9..], &[17, 18, 19]);
    }

    #[test]
    fn configured_auto_keeps_f32_precision() {
        let input = [0.3000f32, 0.3001, 0.3002];
        let mut data = [input, input, input, [0.17, 0.18, 0.19]].concat();
        apply_configured(&AutoOptions::default(), &mut data, 3, u16::MAX, |_| true);
        assert!(data[1] > 0.35 && data[1] < 0.65);
        assert_eq!(&data[9..], &[0.17, 0.18, 0.19]);
    }
}
