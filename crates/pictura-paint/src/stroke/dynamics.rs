//! Dynamics placement: one dab's scatter, jitter, pose, color, and transfer.

use super::*;

/// One dab after the dynamics have placed, sized, turned, coloured, and
/// attenuated it.
pub(super) struct PlacedDab {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) tip: Option<StrokeConfig>,
    pub(super) paint: Rgba,
    pub(super) opacity: f32,
    pub(super) flow: f32,
}

impl Stroke {
    /// One dab of a step, scattered and jittered per the dynamics.
    pub(super) fn placed_dab(
        &mut self,
        cfg: StrokeConfig,
        input: crate::dynamics::DynamicsInput,
        tangent: f32,
        x: f32,
        y: f32,
    ) -> PlacedDab {
        let dab = self.dab_index;
        let initial = self.initial_tangent;
        let dynamics = cfg.dynamics;
        let (sx, sy) = dynamics.scatter_offset(cfg.scatter, input, dab, tangent, &mut self.rng);
        let reach = cfg.diameter as f32 * cfg.scatter as f32 / 100.0;
        let x = x + sx * reach;
        let y = y + sy * reach;

        let size =
            dynamics.size_factor(cfg.size_jitter, input, dab, tangent, initial, &mut self.rng);
        let angle = dynamics.angle(
            cfg.angle_deg,
            cfg.angle_jitter,
            input,
            dab,
            tangent,
            initial,
            &mut self.rng,
        );
        let mut tip_round = dynamics.roundness_percent(
            cfg.roundness,
            cfg.roundness_jitter,
            input,
            dab,
            tangent,
            initial,
            &mut self.rng,
        );
        let (angle, projected_round) = dynamics.projected_tip(angle, tip_round, input);
        tip_round = projected_round;

        let diameter = ((cfg.diameter as f32 * size).round() as u32).max(1);
        let mut tip = None;
        if diameter != cfg.diameter || angle != cfg.angle_deg || tip_round != cfg.roundness {
            let mut changed = cfg;
            changed.diameter = diameter;
            changed.angle_deg = angle;
            changed.roundness = tip_round;
            tip = Some(changed);
        }

        let (opacity, flow) = dynamics.transfer(input, dab, &mut self.rng);
        let paint = if dynamics.color.enabled {
            if dynamics.color.apply_per_tip {
                let base = self.paint;
                let background = cfg.background;
                dynamics.color(base, background, input, dab, &mut self.rng)
            } else {
                self.stroke_color.unwrap_or(self.paint)
            }
        } else {
            self.paint
        };

        self.dab_index = self.dab_index.wrapping_add(1);
        PlacedDab {
            x,
            y,
            tip,
            paint,
            opacity: (cfg.opacity as f32 / 100.0 * opacity).clamp(0.0, 1.0),
            flow: (cfg.flow as f32 / 100.0 * flow).clamp(0.0, 1.0),
        }
    }
}
