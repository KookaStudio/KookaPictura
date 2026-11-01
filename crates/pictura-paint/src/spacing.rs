//! Dab spacing along a stroke.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpacingMode {
    Fixed(u16),
    VelocityDriven,
}

/// Hard ceiling on dabs emitted by a single [`DabPlacer::feed`] call.
pub const MAX_DABS_PER_CALL: usize = 100_000;

/// Walks a stroke sample-by-sample and emits dab centers spaced by a step
/// policy. Residual distance is carried across samples, so spacing stays even
/// when samples arrive unevenly.
pub struct DabPlacer {
    mode: SpacingMode,
    diameter: f32,
    residual: f32,
    last: Option<(f32, f32)>,
    placed_any: bool,
}

impl DabPlacer {
    pub fn new(mode: SpacingMode, diameter: f32) -> Self {
        Self {
            mode,
            diameter,
            residual: 0.0,
            last: None,
            placed_any: false,
        }
    }

    fn step(&self, seg: f32) -> f32 {
        let raw = match self.mode {
            SpacingMode::Fixed(percent) => percent as f32 / 100.0 * self.diameter,
            SpacingMode::VelocityDriven => seg,
        };
        raw.max(0.5)
    }

    /// Feed the next sample; push dab centers into `out`. First call always places one dab at the sample.
    pub fn feed(&mut self, x: f32, y: f32, out: &mut Vec<(f32, f32)>) {
        if !self.placed_any {
            out.push((x, y));
            self.last = Some((x, y));
            self.placed_any = true;
            self.residual = 0.0;
            return;
        }

        let (lx, ly) = self.last.expect("placed_any implies last");
        let dx = x - lx;
        let dy = y - ly;
        let seg = (dx * dx + dy * dy).sqrt();
        if seg <= f32::EPSILON {
            return;
        }

        let step = self.step(seg);
        let mut next_at = (step - self.residual).max(0.0);
        let mut placed = 0usize;
        let mut last_p = 0.0f32;
        while next_at <= seg && placed < MAX_DABS_PER_CALL {
            let t = next_at / seg;
            out.push((lx + dx * t, ly + dy * t));
            last_p = next_at;
            placed += 1;
            next_at += step;
        }

        self.residual = if placed > 0 {
            seg - last_p
        } else {
            self.residual + seg
        };
        self.last = Some((x, y));
    }

    /// Re-emit the last sample for a time-driven dab. The carried residual
    /// stays put, so path spacing resumes as if the dab had not happened.
    pub fn repeat_last(&self, out: &mut Vec<(f32, f32)>) {
        out.extend(self.last);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(mode: SpacingMode, diameter: f32, samples: &[(f32, f32)]) -> usize {
        let mut placer = DabPlacer::new(mode, diameter);
        let mut out = Vec::new();
        for &(x, y) in samples {
            placer.feed(x, y, &mut out);
        }
        out.len()
    }

    #[test]
    fn repeat_last_is_empty_before_any_dab() {
        let mut out = Vec::new();
        DabPlacer::new(SpacingMode::Fixed(25), 12.0).repeat_last(&mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn repeat_last_keeps_path_spacing() {
        let path = [(0.0, 0.0), (5.0, 0.0), (17.0, 0.0), (40.0, 0.0)];
        let walk = |hold: bool| {
            let mut placer = DabPlacer::new(SpacingMode::Fixed(25), 12.0);
            let mut out = Vec::new();
            for &(x, y) in &path {
                placer.feed(x, y, &mut out);
                if hold {
                    placer.repeat_last(&mut Vec::new());
                }
            }
            out
        };
        assert_eq!(walk(true), walk(false));
    }

    #[test]
    fn first_sample_always_places_a_dab() {
        let mut placer = DabPlacer::new(SpacingMode::Fixed(25), 12.0);
        let mut out = Vec::new();
        placer.feed(3.0, 4.0, &mut out);
        assert_eq!(out, vec![(3.0, 4.0)]);
    }

    #[test]
    fn fixed_percent_controls_dab_density() {
        let line = [(0.0, 0.0), (100.0, 0.0)];
        let dense = count(SpacingMode::Fixed(10), 100.0, &line);
        let sparse = count(SpacingMode::Fixed(100), 100.0, &line);
        assert!(dense > sparse, "dense={dense} sparse={sparse}");
        assert!(sparse <= 2, "100px step over 100px: got {sparse}");
        assert!(dense >= 10, "10px steps over 100px: got {dense}");
    }

    #[test]
    fn residual_carries_across_samples() {
        let mut contiguous = Vec::new();
        let mut a = DabPlacer::new(SpacingMode::Fixed(50), 100.0);
        for x in 0..=10 {
            a.feed(x as f32 * 10.0, 0.0, &mut contiguous);
        }
        let mut chunked = Vec::new();
        let mut b = DabPlacer::new(SpacingMode::Fixed(50), 100.0);
        b.feed(0.0, 0.0, &mut chunked);
        b.feed(100.0, 0.0, &mut chunked);
        assert_eq!(contiguous.len(), chunked.len());
    }

    #[test]
    fn identical_points_add_nothing() {
        let mut placer = DabPlacer::new(SpacingMode::Fixed(10), 100.0);
        let mut out = Vec::new();
        placer.feed(5.0, 5.0, &mut out);
        for _ in 0..1000 {
            placer.feed(5.0, 5.0, &mut out);
        }
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn dab_cap_is_respected() {
        let mut placer = DabPlacer::new(SpacingMode::Fixed(1), 1.0);
        let mut out = Vec::new();
        placer.feed(0.0, 0.0, &mut out);
        placer.feed(1.0e9, 0.0, &mut out);
        assert!(out.len() <= MAX_DABS_PER_CALL + 1, "got {}", out.len());
    }

    #[test]
    fn velocity_driven_spaces_by_segment_length() {
        let mut slow = Vec::new();
        let mut a = DabPlacer::new(SpacingMode::VelocityDriven, 12.0);
        for i in 0..=100 {
            a.feed(i as f32, 0.0, &mut slow);
        }
        let mut fast = Vec::new();
        let mut b = DabPlacer::new(SpacingMode::VelocityDriven, 12.0);
        for i in 0..=10 {
            b.feed(i as f32 * 10.0, 0.0, &mut fast);
        }
        assert!(slow.len() > fast.len());
        assert!(fast.len() >= 10);
    }
}
