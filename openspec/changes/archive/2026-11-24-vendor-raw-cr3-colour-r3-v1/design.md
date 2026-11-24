# Design: vendor-raw-cr3-colour-r3-v1

The fit method, acceptance limits and preset placement are #1574's
(`vendor-raw-cr3-colour`), unchanged.

## Samples

raw.pixls.us has nothing new for these bodies: the R3's folder holds only the
grey `_C3A1200.CR3`, and the V1 and R6 Mark III folders hold no further
files. dpreview's sample galleries refuse scripted downloads (a Cloudflare
browser check), so the owner supplied ten CR3s from them in
`~/Downloads/PHOTOSHOP/RAW/Canon`; the run reads a copy in
`~/Downloads/cr3-1579`.

| Body | Sample | Version | Blocks | Median / p90 ΔE | Spread | Verdict |
|---|---|---|---|---|---|---|
| EOS R1 | canon_eos_r1.cr3 | 64 | 488 | 0.97 / 2.62 | 15.6 | fitted, new row |
| EOS R3 | canon_eos_r3.cr3 | 34 | 509 | 2.87 / 7.02 | 30.3 | fitted |
| EOS R6 Mark III | canon_eos_R6_mk3.cr3 | 66 | 505 | 0.90 / 1.82 | 12.6 | fitted, pair reads back |
| EOS R6 Mark III | canon_eos_r6_mk3_pic2.cr3 | 66 | 80 | 4.06 / 12.92 | 33.6 | rejected (median over 4) |
| PowerShot V1 | canon_powershot_v1.cr3 | 65 | 375 | 0.87 / 2.20 | 16.0 | fitted |
| EOS R8 Mark II | canon_eos_R8_mk2.CR3 | 67 | 279 | 4.71 / 9.23 | 19.5 | rejected (median over 4) |
| EOS R6 Mark II | canon_eos_6_mk2.cr3 | 48 | 435 | 0.97 / 3.43 | 15.6 | recorded; spectral row |
| EOS R7 | canon_eos_r7_pic2.cr3 | 48 | 445 | 1.90 / 4.99 | 14.3 | recorded; R7 keeps its #1574 fit |
| EOS R7 | canon_eos_r7.cr3 | 48 | 261 | 4.38 / 8.04 | 22.3 | rejected |
| EOS M50 | canon_eos_m50_02.cr3 | 16 | 401 | 4.38 / 7.17 | 12.7 | rejected |

## Colour-data version 67

The R8 Mark II writes colour-data version 67, which the fit script did not
know (`no daylight white-balance preset`). Its recorded temperatures sit at
words 114 (5200), 119 (7000), 124 (6000) and 129 (3200): version 64's layout,
Daylight at word 110. The codec's as-shot reader already maps 64 to 67 to
word 105. The body still fails the fit (4.71 ΔE median) and moves to the
follow-up.

## Choosing among a body's samples

The generator took the first accepted sample of a body by recorded order
(make, model, file name, case-sensitive), which kept the R6 Mark III's
D55-only fit from `Canon_EOS_R6_Mark_III__IMG_4241.CR3` ahead of the new
pair. It now takes the first sample with both matrices, else the first. Rows
keep their position in the table. No other body has an accepted pair behind
a single-matrix fit, so only the R6 Mark III changes; the R7 keeps its #1574
fit.

## RP and R50: keep the spectral rows

A matrix-to-matrix comparison on synthetic saturated colours cannot decide
this: two camera-JPEG fits of the same R10 differ by 12 ΔE there, because a
fit is constrained only by its scene's colours. Canon's own white-balance
presets give a test independent of the JPEG. They are the sensor's raw
response to 5200 K and 3200 K light, and a sound XYZ-to-camera matrix
predicts them (the pair interpolated in mired, CIE daylight at 5200 K,
Planckian at 3200 K). Largest log error of the four ratios (R/G, B/G at both
lights):

| Spectral row | Sample | Error |
|---|---|---|
| EOS Rebel SL3 (200D II) | Sedona2020_SL3 | 0.068 |
| EOS R50 (R10 data) | R50 RAW | 0.082 |
| EOS RP | RP RAW | 0.085 |
| EOS R8 (R6m2 data) | R8 RAW | 0.086 |
| EOS R | R RAW | 0.087 |
| EOS R6 Mark II | r6m2 0346 | 0.088 |
| EOS 250D (200D II data) | 250D RAW | 0.091 |
| EOS R6 | R6 RAW | 0.092 |
| EOS R5 Mark II | R5m2 APS-C | 0.098 |
| EOS R10 | IMG_6310 | 0.099 |
| EOS R5 | R5 RAW | 0.136 |

Every spectral row errs the same way (R/G low, B/G high by a few per cent),
a shared offset between Canon's preset definition and the CIE whites. The RP
and R50 sit inside the band, so neither row looks unsound.

The R50's presets match the R10's: Daylight R/G 0.579 and 0.579, B/G 0.590
and 0.588; Tungsten 0.819 / 0.417 and 0.813 / 0.410. Bodies with other
sensors differ by far more (R7 0.569 / 0.652, R6 0.545 / 0.642). The R50 V
matches too (0.580 / 0.587). This supports rawtoaces-data's R50 → R10 alias.

The R10 and R50 share one spectral row and one sensor, yet sit 2.1 and 11.5
ΔE from their own sample JPEGs (#1574's validation table). So the gap comes
from how each sample's JPEG was rendered, not from the spectral data. Both
rows stay.

## Follow-up

Bodies still without colour go to a new issue: the R8 Mark II (fit 4.71 ΔE),
and the bodies with no usable sample (PowerShot G1 X Mark III and other
PowerShots). The M6 Mark II and G7 X Mark III keep their proxies until a
colourful sample of their own turns up.

#1579 listed the EOS R5 C among the bodies without a sample; it already
takes the R5's spectral row through rawtoaces-data's alias, so the tests use
the PowerShot G1 X Mark III as their colourless body.

## Splitting the table

The three new rows and the R6 Mark III's tungsten matrix took the generated
`camera_matrices.rs` to 1218 lines, over the 1200-line cap. The generator
now writes the camera-JPEG rows and their proxies to `camera_jpeg_matrices.rs`
(`JPEG_CAMERAS`), and the lookup searches the spectral table first, then that
one, the same order as before. The milestone-name exemption moves with the
M50 / M200 keys to the new file.
