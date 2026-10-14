use super::*;

fn params() -> LensParams {
    LensParams {
        focal: 0,
        invert: false,
        shape: IrisShape::Hexagon,
        radius: 8,
        curvature: 0,
        rotation: 0.0,
        specular_brightness: 0,
        specular_threshold: 255,
        noise_amount: 0,
        noise_distribution: NoiseDistribution::Uniform,
        monochromatic: false,
        faster: false,
        seed: 1,
    }
}

/// A left/right step edge so a blur has something to smooth.
fn step_buffer(w: u32, h: u32) -> PixelBuffer {
    let n = (w * h) as usize;
    let mut data = vec![0u8; n * 3];
    for y in 0..h {
        for x in 0..w {
            let v = if x < w / 2 { 40 } else { 200 };
            let i = (y * w + x) as usize;
            data[i] = v;
            data[n + i] = v;
            data[2 * n + i] = v;
        }
    }
    PixelBuffer {
        width: w,
        height: h,
        channels: 3,
        data: data.into(),
    }
}

/// A 5×5 bright spot at the centre.
fn spot(w: u32, h: u32, value: u8) -> PixelBuffer {
    let mut buf = PixelBuffer::new(w, h, 3);
    let n = buf.pixel_count();
    for y in (h / 2 - 2)..=(h / 2 + 2) {
        for x in (w / 2 - 2)..=(w / 2 + 2) {
            let i = (y * w + x) as usize;
            buf.data[i] = value;
            buf.data[n + i] = value;
            buf.data[2 * n + i] = value;
        }
    }
    buf
}

fn px(buf: &PixelBuffer, x: u32, y: u32) -> [u8; 3] {
    let w = buf.width as usize;
    let n = buf.pixel_count();
    let i = y as usize * w + x as usize;
    [buf.data[i], buf.data[n + i], buf.data[2 * n + i]]
}

fn plane(buf: &PixelBuffer, c: usize) -> &[u8] {
    let n = buf.pixel_count();
    &buf.data[c * n..(c + 1) * n]
}

fn depth(w: u32, h: u32, f: impl Fn(u32, u32) -> u8) -> DepthMap {
    let mut samples = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            samples.push(f(x, y));
        }
    }
    DepthMap::new(w, h, samples).unwrap()
}

/// Pixels on `y` that are neither step value, so a blur reached them.
fn soft_pixels(buf: &PixelBuffer, y: u32) -> usize {
    (0..buf.width)
        .filter(|&x| {
            let v = px(buf, x, y)[0];
            v != 40 && v != 200
        })
        .count()
}

#[test]
fn in_focus_depth_stays_sharp() {
    let before = step_buffer(16, 8);
    let mut buf = before.clone();
    let map = depth(16, 8, |_x, _y| 128);
    let mut p = params();
    p.focal = 128;
    p.radius = 10;
    lens_blur(&mut buf, Some(&map), &p).unwrap();
    assert_eq!(buf.data, before.data, "focal depth must not blur");
}

#[test]
fn a_depth_ramp_blurs_the_far_side() {
    let (w, h) = (16, 16);
    let before = step_buffer(w, h);
    let mut buf = before.clone();
    // Black (near) at the bottom, white (far) at the top.
    let map = depth(w, h, |_x, y| (((h - 1 - y) * 255) / (h - 1)) as u8);
    let mut p = params();
    p.radius = 8;
    lens_blur(&mut buf, Some(&map), &p).unwrap();
    for x in 0..w {
        assert_eq!(px(&buf, x, h - 1), px(&before, x, h - 1), "near edge moved");
    }
    assert_eq!(soft_pixels(&buf, h - 1), 0);
    assert!(soft_pixels(&buf, 0) > 0, "far edge did not soften");
}

#[test]
fn a_point_light_repeats_the_iris_shape_and_its_rotation() {
    let (w, h) = (33, 33);
    let base = spot(w, h, 255);
    let mut p = params();
    p.radius = 12;
    p.shape = IrisShape::Triangle;
    // A triangle vertex points right: the pixel left of the light sees it at
    // the vertex and gathers energy; the pixel above sees an edge midpoint.
    let mut tri = base.clone();
    lens_blur(&mut tri, None, &p).unwrap();
    assert!(px(&tri, 4, 16)[0] > 0, "the vertex direction is empty");
    assert_eq!(px(&tri, 16, 4)[0], 0, "the edge midpoint carried energy");
    // Curvature 100 rounds the blades into a circle, filling that direction.
    p.curvature = 100;
    let mut disc = base.clone();
    lens_blur(&mut disc, None, &p).unwrap();
    assert!(px(&disc, 16, 4)[0] > 0, "the disc is not round");
    // Rotating the iris by 90° turns a vertex up.
    p.curvature = 0;
    p.rotation = 90.0;
    let mut rotated = base.clone();
    lens_blur(&mut rotated, None, &p).unwrap();
    assert!(px(&rotated, 16, 4)[0] > 0, "the rotated vertex is empty");
    assert_eq!(px(&rotated, 4, 16)[0], 0, "the old vertex stayed lit");
}

#[test]
fn specular_highlights_bloom_only_above_the_threshold() {
    let (w, h) = (33, 33);
    let mut bright = PixelBuffer::new(w, h, 3);
    let n = bright.pixel_count();
    for y in 15..18 {
        for x in 15..18 {
            let i = (y * w + x) as usize;
            bright.data[i] = 255;
            bright.data[n + i] = 255;
            bright.data[2 * n + i] = 255;
        }
    }
    let mut p = params();
    p.radius = 8;
    p.specular_threshold = 128;
    let mut plain = bright.clone();
    lens_blur(&mut plain, None, &p).unwrap();
    p.specular_brightness = 100;
    let mut bloom = bright.clone();
    lens_blur(&mut bloom, None, &p).unwrap();
    let sum = |b: &PixelBuffer| b.data.iter().map(|&v| v as u64).sum::<u64>();
    assert!(sum(&bloom) > sum(&plain), "the highlight did not bloom");

    // A dim spot below the threshold is unaffected by the boost.
    let mut dim = PixelBuffer::new(w, h, 3);
    for y in 15..18 {
        for x in 15..18 {
            let i = (y * w + x) as usize;
            dim.data[i] = 200;
            dim.data[n + i] = 200;
            dim.data[2 * n + i] = 200;
        }
    }
    p.specular_threshold = 250;
    let mut boosted = dim.clone();
    lens_blur(&mut boosted, None, &p).unwrap();
    p.specular_brightness = 0;
    let mut unboosted = dim.clone();
    lens_blur(&mut unboosted, None, &p).unwrap();
    assert_eq!(boosted.data, unboosted.data, "a dim spot bloomed");
}

#[test]
fn noise_is_seeded_and_monochromatic_keeps_channels_equal() {
    let base = PixelBuffer::new(8, 8, 3);
    let mut p = params();
    p.radius = 0;
    p.noise_amount = 47;
    p.monochromatic = true;
    let mut a = base.clone();
    lens_blur(&mut a, None, &p).unwrap();
    let mut b = base.clone();
    lens_blur(&mut b, None, &p).unwrap();
    assert_eq!(a.data, b.data, "equal seeds must replay");
    for i in 0..64 {
        assert_eq!(plane(&a, 0)[i], plane(&a, 1)[i]);
        assert_eq!(plane(&a, 1)[i], plane(&a, 2)[i]);
    }
    assert!(a.data.iter().zip(base.data.iter()).any(|(x, y)| x != y));
    p.seed = 2;
    let mut c = base.clone();
    lens_blur(&mut c, None, &p).unwrap();
    assert_ne!(a.data, c.data, "a new seed must re-roll the grain");
}

#[test]
fn alpha_takes_the_defocus_without_bloom_or_noise() {
    let (w, h) = (9, 9);
    let n = (w * h) as usize;
    let ramp: Vec<u8> = (0..n).map(|i| (i * 11) as u8).collect();
    let mut data = vec![0u8; n * 4];
    for i in 0..n {
        data[i] = 50;
        data[n + i] = 80;
        data[2 * n + i] = 120;
        data[3 * n + i] = ramp[i];
    }
    let mut rgba = PixelBuffer {
        width: w,
        height: h,
        channels: 4,
        data: data.into(),
    };
    let mut p = params();
    p.radius = 4;
    p.noise_amount = 30;
    p.specular_brightness = 100;
    p.specular_threshold = 0;
    lens_blur(&mut rgba, None, &p).unwrap();

    // The same plane as colour, with the bloom and the noise off.
    let mut grey = PixelBuffer {
        width: w,
        height: h,
        channels: 3,
        data: ramp.repeat(3).into(),
    };
    p.noise_amount = 0;
    p.specular_brightness = 0;
    lens_blur(&mut grey, None, &p).unwrap();
    assert_ne!(&rgba.data[3 * n..], &ramp[..], "alpha is defocused");
    assert_eq!(&rgba.data[3 * n..], &grey.data[..n]);
}

#[test]
fn a_small_depth_map_stretches_over_the_buffer() {
    let (w, h) = (8, 16);
    let before = step_buffer(w, h);
    let map = DepthMap::new(1, 2, vec![0, 255]).unwrap();
    let mut buf = before.clone();
    let mut p = params();
    p.radius = 8;
    lens_blur(&mut buf, Some(&map), &p).unwrap();
    for x in 0..w {
        assert_eq!(px(&buf, x, 0), px(&before, x, 0), "the near row moved");
    }
    assert_eq!(soft_pixels(&buf, 0), 0);
    assert!(soft_pixels(&buf, h - 1) > 0, "the far row stayed sharp");
    assert!(soft_pixels(&buf, h / 2) > 0, "the stretch stepped");
}

#[test]
fn a_none_map_is_a_uniform_far_depth() {
    let mut a = step_buffer(16, 8);
    let mut b = a.clone();
    let map = depth(16, 8, |_x, _y| 255);
    let p = params();
    lens_blur(&mut a, None, &p).unwrap();
    lens_blur(&mut b, Some(&map), &p).unwrap();
    assert_eq!(a.data, b.data);
}

#[test]
fn invert_flips_the_depth() {
    let (w, h) = (16, 16);
    let map = depth(w, h, |_x, y| if y < h / 2 { 0 } else { 255 });
    let mut p = params();
    p.radius = 8;
    let mut normal = step_buffer(w, h);
    lens_blur(&mut normal, Some(&map), &p).unwrap();
    p.invert = true;
    let mut inverted = step_buffer(w, h);
    lens_blur(&mut inverted, Some(&map), &p).unwrap();
    assert_eq!(soft_pixels(&normal, 0), 0, "the near half should be sharp");
    assert!(soft_pixels(&normal, h - 1) > 0);
    assert!(soft_pixels(&inverted, 0) > 0, "invert did not flip");
    assert_eq!(soft_pixels(&inverted, h - 1), 0);
}

#[test]
fn invalid_parameters_are_refused_without_mutation() {
    let mut buf = step_buffer(8, 8);
    let before = buf.data.to_vec();
    for mutate in [
        |p: &mut LensParams| p.radius = 101,
        |p: &mut LensParams| p.rotation = f64::NAN,
        |p: &mut LensParams| p.curvature = 101,
        |p: &mut LensParams| p.specular_brightness = 101,
        |p: &mut LensParams| p.noise_amount = 101,
    ] {
        let mut p = params();
        mutate(&mut p);
        assert!(lens_blur(&mut buf, None, &p).is_err());
    }
    assert_eq!(buf.data, before);
}

#[test]
fn tiny_images_do_not_panic() {
    for (w, h) in [(1, 1), (1, 7), (7, 1)] {
        let mut buf = PixelBuffer::new(w, h, 3);
        let mut p = params();
        p.radius = 5;
        p.noise_amount = 10;
        lens_blur(&mut buf, None, &p).unwrap();
    }
}
