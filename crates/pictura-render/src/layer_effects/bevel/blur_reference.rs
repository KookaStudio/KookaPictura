use std::cell::Cell;

thread_local! {
    static SERIAL: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<[usize; 2]> = const { Cell::new([0; 2]) };
}

pub(crate) fn using_serial() -> bool {
    SERIAL.get()
}

pub(crate) fn record_call(serial: bool) {
    let mut calls = CALLS.get();
    calls[usize::from(!serial)] += 1;
    CALLS.set(calls);
}

pub(crate) fn with_serial<T>(serial: bool, run: impl FnOnce() -> T) -> (T, [usize; 2]) {
    struct Reset(bool, [usize; 2]);
    impl Drop for Reset {
        fn drop(&mut self) {
            SERIAL.set(self.0);
            CALLS.set(self.1);
        }
    }
    let _reset = Reset(SERIAL.replace(serial), CALLS.replace([0; 2]));
    let result = run();
    (result, CALLS.get())
}

// Independent of the new helper and its call-site sigma conversion.
pub(crate) fn former_blur(src: &[f32], w: usize, h: usize, radius: f64) -> Vec<f32> {
    let kernel = pictura_filters::kernel::gaussian_kernel(
        pictura_filters::kernel::sigma_from_radius(radius),
    );
    let r = (kernel.len() / 2) as isize;
    let pass = |src: &[f32], horizontal: bool| -> Vec<f32> {
        let mut out = vec![0.0f32; src.len()];
        for y in 0..h {
            for x in 0..w {
                let mut acc = 0.0f64;
                for (i, k) in kernel.iter().enumerate() {
                    let o = i as isize - r;
                    let (sx, sy) = if horizontal {
                        ((x as isize + o).clamp(0, w as isize - 1) as usize, y)
                    } else {
                        (x, (y as isize + o).clamp(0, h as isize - 1) as usize)
                    };
                    acc += k * src[sy * w + sx] as f64;
                }
                out[y * w + x] = acc as f32;
            }
        }
        out
    };
    pass(&pass(src, true), false)
}

pub(crate) fn former_distance(seed: &[bool], w: usize, h: usize) -> Vec<f32> {
    const FAR: f64 = 1.0e12;
    let mut grid: Vec<f64> = seed.iter().map(|&s| if s { 0.0 } else { FAR }).collect();
    let mut line = Vec::new();
    for x in 0..w {
        line.clear();
        line.extend((0..h).map(|y| grid[y * w + x]));
        for (y, v) in super::squared_distance_1d(&line).into_iter().enumerate() {
            grid[y * w + x] = v;
        }
    }
    for y in 0..h {
        let row = super::squared_distance_1d(&grid[y * w..(y + 1) * w]);
        grid[y * w..(y + 1) * w].copy_from_slice(&row);
    }
    grid.into_iter().map(|d| d.sqrt() as f32).collect()
}
