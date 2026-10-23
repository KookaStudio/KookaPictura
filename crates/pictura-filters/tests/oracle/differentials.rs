//! The filters with a faithful ImageMagick operator, diffed differentially.

use pictura_filters::Filter;

use crate::common::differential;

#[test]
fn gaussian_blur_matches_imagemagick() {
    differential(
        &Filter::GaussianBlur { radius: 3.0 },
        &["--op", "gaussian", "--sigma", "1.0"],
        0,
        "GaussianBlur radius 3.0",
    );
}

#[test]
fn box_blur_matches_imagemagick() {
    differential(
        &Filter::BoxBlur { radius: 3 },
        &["--op", "box", "--radius", "3"],
        0,
        "BoxBlur radius 3",
    );
}

#[test]
fn blur_matches_imagemagick() {
    differential(
        &Filter::Blur,
        &["--op", "gaussian", "--sigma", "0.7"],
        0,
        "Blur",
    );
}

#[test]
fn blur_more_matches_imagemagick() {
    differential(
        &Filter::BlurMore,
        &["--op", "gaussian", "--sigma", "2.0"],
        1,
        "BlurMore",
    );
}

#[test]
fn median_matches_imagemagick() {
    differential(
        &Filter::Median { radius: 1 },
        &["--op", "median", "--radius", "1"],
        0,
        "Median radius 1",
    );
}

#[test]
fn unsharp_mask_matches_imagemagick() {
    differential(
        &Filter::UnsharpMask {
            amount: 150.0,
            radius: 3.0,
            threshold: 0,
        },
        &[
            "--op",
            "unsharp",
            "--sigma",
            "1.0",
            "--amount",
            "150",
            "--threshold",
            "0",
        ],
        6,
        "UnsharpMask radius 3.0 amount 150 threshold 0",
    );
}

#[test]
fn sharpen_matches_imagemagick() {
    differential(
        &Filter::Sharpen,
        &[
            "--op",
            "unsharp",
            "--sigma",
            "1.0",
            "--amount",
            "50",
            "--threshold",
            "0",
        ],
        2,
        "Sharpen",
    );
}

#[test]
fn sharpen_more_matches_imagemagick() {
    differential(
        &Filter::SharpenMore,
        &[
            "--op",
            "unsharp",
            "--sigma",
            "1.0",
            "--amount",
            "100",
            "--threshold",
            "0",
        ],
        3,
        "SharpenMore",
    );
}

/// `Filter::Custom` kernel used by the differential test: a 5x5 sharpening /
/// edge kernel with a non-trivial sum, so both the divisor and the bias are
/// exercised. Sum = 29.
const CUSTOM_TEST_KERNEL: [[f64; 5]; 5] = [
    [0.0, 0.0, -1.0, 0.0, 0.0],
    [0.0, -1.0, 4.0, -1.0, 0.0],
    [-1.0, 4.0, 20.0, 4.0, -1.0],
    [0.0, -1.0, 4.0, -1.0, 0.0],
    [0.0, 0.0, -1.0, 0.0, 0.0],
];

#[test]
fn maximum_matches_imagemagick() {
    differential(
        &Filter::Maximum { radius: 2 },
        &["--op", "maximum", "--radius", "2"],
        0,
        "Maximum radius 2",
    );
}

#[test]
fn minimum_matches_imagemagick() {
    differential(
        &Filter::Minimum { radius: 2 },
        &["--op", "minimum", "--radius", "2"],
        0,
        "Minimum radius 2",
    );
}

#[test]
fn offset_wrap_matches_imagemagick() {
    differential(
        &Filter::Offset {
            horizontal: 3,
            vertical: 2,
            wrap: true,
            repeat: false,
            background: [0, 0, 0],
            transparent: false,
        },
        &["--op", "roll", "--horizontal", "3", "--vertical", "2"],
        0,
        "Offset wrap (3,2)",
    );
}

#[test]
fn custom_matches_imagemagick() {
    differential(
        &Filter::Custom {
            kernel: CUSTOM_TEST_KERNEL,
            scale: 4.0,
            offset: 8.0,
        },
        &[
            "--op",
            "convolve",
            "--kernel",
            "0,0,-1,0,0,0,-1,4,-1,0,-1,4,20,4,-1,0,-1,4,-1,0,0,0,-1,0,0",
            "--kernel-scale",
            "4",
            "--kernel-offset",
            "8",
        ],
        0,
        "Custom edge kernel scale 4 offset 8",
    );
}

#[test]
fn solarize_matches_imagemagick() {
    differential(
        &Filter::Solarize,
        &["--op", "solarize", "--threshold-percent", "50"],
        0,
        "Solarize 50%",
    );
}

/// M8: `Mosaic` with a cell that divides the image is an exact top-left block
/// mean, which `-filter box -resize` down + `-filter point -resize` up matches
/// bit-exactly (measured max delta 0).
#[test]
fn mosaic_matches_imagemagick() {
    differential(
        &Filter::Mosaic { cell_size: 4 },
        &["--op", "mosaic", "--cell", "4"],
        0,
        "Mosaic cell 4",
    );
}

#[test]
fn shape_blur_matches_imagemagick() {
    for (shape, radius) in [(1, 5), (2, 4)] {
        let kernel = pictura_filters::blur::shape_kernel(shape, radius);
        let weights: Vec<String> = kernel.iter().map(u8::to_string).collect();
        let arg = format!("{}:{}", 2 * radius + 1, weights.join(","));
        differential(
            &Filter::ShapeBlur { shape, radius },
            &["--op", "shape", "--shape-kernel", &arg],
            0,
            &format!("ShapeBlur shape {shape} radius {radius}"),
        );
    }
}
