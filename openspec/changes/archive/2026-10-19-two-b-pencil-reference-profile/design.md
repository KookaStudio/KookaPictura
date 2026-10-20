# Design: two-b-pencil-reference-profile

## Measurements

The screenshot's pixel grid has a period of 6.74 screen px. Sampling every
cell's centre gives document pixels; coverage is `(255 − grey) / 184` for
`#47484a` on white. The darkest pixel is (72, 75, 76), which is the full
colour, so the tone variation is entirely grain.

| Metric (straight horizontal stroke) | Reference | Before | After |
|---|---|---|---|
| Footprint (coverage > 5 %) | 7.0 px | 8.9 px | ~7 px |
| Effective width (Σ coverage) | 3.7 px | 6.0 px | 3.5 px |
| Core mean (coverage > 0.55) | 0.73 | 0.74 | 0.72 |
| Core p10 / p90 | 0.60 / 0.84 | 0.60 / 0.86 | 0.60 / 0.84 |
| 5×5 residual sd | 0.064 | 0.081 | 0.069 |
| Residual lag-1 correlation | 0.38 | 0.24 | 0.39 |

## Decisions

- **Tip.** A sweep over diameter 5–9, hardness 0–65, spacing 8–35 %, and
  size jitter 0/20 % scored each mean cross-section against the reference.
  At 100 % flow, overlapping dabs saturate (`1 − (1 − a)(1 − c)`), so a
  dense or large tip turns flat-topped; 8 px, 25 % hardness, and 25 % spacing
  keep the soft shoulders without beading. Size jitter stays at 20 %.
- **Grain.** `0.2·fine + 0.35·mid + 0.45·broad` with contrast 1.3 (was
  `0.3 / 0.3 / 0.4` and 1.5) brings the residual sd and correlation to the
  reference while keeping the core mean and percentiles. The weights still sum
  to 1, so the grain-strength coverage targets (75 % / 60 %) hold.
- The reference's vertical strokes are hand-drawn somewhat wider and lighter
  than its horizontal ones; the fit favours the cleaner horizontal stroke.

`ponytail:` fitted to one screenshot; Photoshop's own accumulation (where a
dab only raises coverage toward its own alpha) is not modelled.
