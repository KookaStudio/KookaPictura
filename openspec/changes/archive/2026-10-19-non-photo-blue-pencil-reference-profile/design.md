# Design: non-photo-blue-pencil-reference-profile

## Measurements

The grid period is 6.74 screen px. Coverage is `(255 − R) / 140`: red carries
the most range for `#73d6ff`. The darkest pixel is (122, 214, 254), about 95 %
of the colour. Straight segments: the second horizontal line and both vertical
lines.

| Metric | Reference | Before | After |
|---|---|---|---|
| Mean cross-section (±3 px) | `.03 .09 .44 .68 .44 .04 .02` | `.58 .63 .64 .63 .63 .63 .59` | `.00 .04 .50 .71 .49 .05 .00` |
| Effective width | 1.80 px | 4.83 px | 1.79 px |
| Footprint (> 5 %) | 3.5 px | 8.6 px | 3.3 px |
| Peak mean / sd | 0.739 / 0.101 | 0.776 / 0.112 | 0.725 / 0.097 |
| Peak p10 / p90 | 0.59 / 0.84 | 0.61 / 0.93 | 0.61 / 0.84 |

## Decisions

- A sweep over diameter 3–5, hardness 0–100, spacing 8–30 %, size jitter
  0–20 %, and grain 50–75 scored each render against the reference. The top
  fits were statistically tied. 4 px, 0 % hardness, 20 % spacing, and grain 55
  matched the peak spread best. Size jitter does nothing visible at 4 px, so
  it stays 0.
- The grain noise itself is unchanged; the 2B fit already set its clump
  weights.

`ponytail:` fitted to one screenshot; non-parity.
