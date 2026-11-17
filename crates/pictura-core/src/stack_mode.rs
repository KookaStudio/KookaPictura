//! Stack-mode reductions over a stack of aligned RGBA frames.

use crate::PixelBuffer;

/// The per-channel reduction a smart object's image stack applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StackMode {
    Maximum,
    Minimum,
    Mean,
    Median,
    Summation,
    Range,
    Variance,
    StandardDeviation,
    Skewness,
    Kurtosis,
    Entropy,
}

impl StackMode {
    /// Every mode, in the stable code order used by the codec marker.
    pub const ALL: [Self; 11] = [
        Self::Maximum,
        Self::Minimum,
        Self::Mean,
        Self::Median,
        Self::Summation,
        Self::Range,
        Self::Variance,
        Self::StandardDeviation,
        Self::Skewness,
        Self::Kurtosis,
        Self::Entropy,
    ];

    /// The menu and history label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Maximum => "Maximum",
            Self::Minimum => "Minimum",
            Self::Mean => "Mean",
            Self::Median => "Median",
            Self::Summation => "Summation",
            Self::Range => "Range",
            Self::Variance => "Variance",
            Self::StandardDeviation => "Standard Deviation",
            Self::Skewness => "Skewness",
            Self::Kurtosis => "Kurtosis",
            Self::Entropy => "Entropy",
        }
    }

    /// The stable one-byte code used by the `PiSt` marker; `0` is reserved for
    /// no mode.
    pub const fn code(self) -> u8 {
        match self {
            Self::Maximum => 1,
            Self::Minimum => 2,
            Self::Mean => 3,
            Self::Median => 4,
            Self::Summation => 5,
            Self::Range => 6,
            Self::Variance => 7,
            Self::StandardDeviation => 8,
            Self::Skewness => 9,
            Self::Kurtosis => 10,
            Self::Entropy => 11,
        }
    }

    /// The mode for a `PiSt` code, or `None` for an unknown code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Maximum),
            2 => Some(Self::Minimum),
            3 => Some(Self::Mean),
            4 => Some(Self::Median),
            5 => Some(Self::Summation),
            6 => Some(Self::Range),
            7 => Some(Self::Variance),
            8 => Some(Self::StandardDeviation),
            9 => Some(Self::Skewness),
            10 => Some(Self::Kurtosis),
            11 => Some(Self::Entropy),
            _ => None,
        }
    }
}

/// The entropy of an 8-bit channel is at most `log2(256) = 8` bits; scale it to
/// the 0-255 channel range.
const ENTROPY_SCALE: f64 = 255.0 / 8.0;

/// Reduce `frames` per pixel and channel.
///
/// Frames are straight-alpha RGBA buffers of the same size; at a pixel a frame
/// contributes a channel when its alpha is above zero. The output alpha is the
/// maximum contributing alpha, and a pixel covered by no frame is transparent
/// black. The returned buffer is always 4-channel.
pub fn reduce_stack(frames: &[PixelBuffer], mode: StackMode) -> PixelBuffer {
    let Some(first) = frames.first() else {
        return PixelBuffer::new(0, 0, 4);
    };
    let (width, height) = (first.width, first.height);
    if width == 0 || height == 0 {
        return PixelBuffer::new(0, 0, 4);
    }
    let plane = width as usize * height as usize;
    let mut out = PixelBuffer::new(width, height, 4);
    let mut samples = Vec::with_capacity(frames.len());
    for i in 0..plane {
        let mut alpha = 0u8;
        for frame in frames {
            if frame.width == width && frame.height == height && frame.data.len() >= plane * 4 {
                alpha = alpha.max(frame.data[3 * plane + i]);
            }
        }
        if alpha == 0 {
            continue;
        }
        for channel in 0..3 {
            samples.clear();
            for frame in frames {
                if frame.width != width
                    || frame.height != height
                    || frame.data.len() < plane * 4
                    || frame.data[3 * plane + i] == 0
                {
                    continue;
                }
                samples.push(frame.data[channel * plane + i]);
            }
            out.data[channel * plane + i] = reduce_channel(&mut samples, mode);
        }
        out.data[3 * plane + i] = alpha;
    }
    out
}

fn reduce_channel(values: &mut [u8], mode: StackMode) -> u8 {
    if values.is_empty() {
        return 0;
    }
    let n = values.len() as f64;
    match mode {
        StackMode::Maximum => *values.iter().max().expect("non-empty"),
        StackMode::Minimum => *values.iter().min().expect("non-empty"),
        StackMode::Mean => {
            let sum: f64 = values.iter().map(|&v| f64::from(v)).sum();
            clamp_round(sum / n)
        }
        StackMode::Median => {
            values.sort_unstable();
            let len = values.len();
            if len % 2 == 1 {
                values[len / 2]
            } else {
                clamp_round((f64::from(values[len / 2 - 1]) + f64::from(values[len / 2])) / 2.0)
            }
        }
        StackMode::Summation => {
            let sum: u64 = values.iter().map(|&v| u64::from(v)).sum();
            sum.min(255) as u8
        }
        StackMode::Range => {
            let max = *values.iter().max().expect("non-empty");
            let min = *values.iter().min().expect("non-empty");
            max - min
        }
        StackMode::Variance
        | StackMode::StandardDeviation
        | StackMode::Skewness
        | StackMode::Kurtosis => moments(values, mode),
        StackMode::Entropy => entropy(values),
    }
}

fn moments(values: &[u8], mode: StackMode) -> u8 {
    let len = values.len();
    if len < 2 {
        return 0;
    }
    let n = len as f64;
    let mean = values.iter().map(|&v| f64::from(v)).sum::<f64>() / n;
    let squares: f64 = values
        .iter()
        .map(|&v| {
            let d = f64::from(v) - mean;
            d * d
        })
        .sum();
    let variance = squares / (n - 1.0);
    if matches!(mode, StackMode::Variance) {
        return clamp_round(variance);
    }
    let stddev = variance.sqrt();
    if stddev == 0.0 {
        return 0;
    }
    if matches!(mode, StackMode::StandardDeviation) {
        return clamp_round(stddev);
    }
    if matches!(mode, StackMode::Skewness) {
        let third: f64 = values
            .iter()
            .map(|&v| {
                let d = f64::from(v) - mean;
                d * d * d
            })
            .sum();
        return clamp_round((third / ((n - 1.0) * stddev.powi(3))).max(0.0));
    }
    let fourth: f64 = values
        .iter()
        .map(|&v| {
            let d = f64::from(v) - mean;
            d * d * d * d
        })
        .sum();
    clamp_round(fourth / ((n - 1.0) * stddev.powi(4)))
}

fn entropy(values: &mut [u8]) -> u8 {
    values.sort_unstable();
    let n = values.len() as f64;
    let mut bits = 0.0;
    let mut i = 0;
    while i < values.len() {
        let mut j = i + 1;
        while j < values.len() && values[j] == values[i] {
            j += 1;
        }
        let p = (j - i) as f64 / n;
        bits -= p * p.log2();
        i = j;
    }
    clamp_round(bits * ENTROPY_SCALE)
}

fn clamp_round(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SmartObject;

    fn pixel(value: u8, alpha: u8) -> PixelBuffer {
        let mut frame = PixelBuffer::new(1, 1, 4);
        let data = frame.data.as_mut_slice();
        data[0] = value;
        data[1] = value;
        data[2] = value;
        data[3] = alpha;
        frame
    }

    fn reduced(frames: &[PixelBuffer], mode: StackMode) -> [u8; 4] {
        let out = reduce_stack(frames, mode);
        std::array::from_fn(|c| out.data[c])
    }

    #[test]
    fn stack_mode_defaults_and_equality() {
        let mut object = SmartObject::default();
        assert_eq!(object.stack_mode, None);
        object.stack_mode = Some(StackMode::Median);
        assert_ne!(object, SmartObject::default());
    }

    #[test]
    fn reduce_stack_matches_each_formula() {
        // 0, 0, 3: mean 1, variance 3, stddev 1.73, skewness 0.58, kurtosis 1.
        let frames = [pixel(0, 255), pixel(0, 255), pixel(3, 255)];
        let expected = [
            (StackMode::Maximum, 3),
            (StackMode::Minimum, 0),
            (StackMode::Mean, 1),
            (StackMode::Median, 0),
            (StackMode::Summation, 3),
            (StackMode::Range, 3),
            (StackMode::Variance, 3),
            (StackMode::StandardDeviation, 2),
            (StackMode::Skewness, 1),
            (StackMode::Kurtosis, 1),
            (StackMode::Entropy, 29),
        ];
        for (mode, value) in expected {
            assert_eq!(
                reduced(&frames, mode),
                [value, value, value, 255],
                "{mode:?}"
            );
        }
    }

    #[test]
    fn reduce_stack_even_median_averages_middle_samples() {
        let frames = [pixel(0, 255), pixel(1, 255), pixel(2, 255), pixel(3, 255)];
        assert_eq!(reduced(&frames, StackMode::Median), [2, 2, 2, 255]);
    }

    #[test]
    fn reduce_stack_guards_single_and_zero_deviation_samples() {
        for frames in [vec![pixel(5, 255)], vec![pixel(5, 255), pixel(5, 255)]] {
            for mode in [
                StackMode::Variance,
                StackMode::StandardDeviation,
                StackMode::Skewness,
                StackMode::Kurtosis,
            ] {
                assert_eq!(reduced(&frames, mode), [0, 0, 0, 255], "{mode:?}");
            }
        }
    }

    #[test]
    fn reduce_stack_scales_entropy_to_the_channel_range() {
        let frames = [
            pixel(0, 255),
            pixel(85, 255),
            pixel(170, 255),
            pixel(255, 255),
        ];
        assert_eq!(reduced(&frames, StackMode::Entropy), [64, 64, 64, 255]);
    }

    #[test]
    fn reduce_stack_clamps_summation_and_moments() {
        let frames = [pixel(100, 255), pixel(200, 255)];
        assert_eq!(reduced(&frames, StackMode::Summation), [255, 255, 255, 255]);
        assert_eq!(reduced(&frames, StackMode::Variance), [255, 255, 255, 255]);
        let left_skewed = [
            pixel(0, 255),
            pixel(255, 255),
            pixel(255, 255),
            pixel(255, 255),
        ];
        assert_eq!(reduced(&left_skewed, StackMode::Skewness), [0, 0, 0, 255]);
    }

    #[test]
    fn reduce_stack_excludes_transparent_samples_and_sets_max_alpha() {
        let frames = [pixel(99, 0), pixel(10, 128), pixel(30, 255)];
        assert_eq!(reduced(&frames, StackMode::Mean), [20, 20, 20, 255]);
        let empty = [pixel(99, 0), pixel(10, 0)];
        assert_eq!(reduced(&empty, StackMode::Maximum), [0, 0, 0, 0]);
    }

    #[test]
    fn stack_mode_codes_round_trip() {
        for mode in StackMode::ALL {
            assert_eq!(StackMode::from_code(mode.code()), Some(mode));
            assert!(!mode.label().is_empty());
        }
        assert_eq!(StackMode::from_code(0), None);
        assert_eq!(StackMode::from_code(12), None);
    }
}
