# Proposal: colour for the Canon EOS R3, PowerShot V1 and weak CR3 rows (#1579)

## Why

#1574 fitted CR3 bodies without spectral data from their own camera JPEGs,
but the only raw.pixls.us samples of the EOS R3 and PowerShot V1 show grey
scenes (a*b* spread 1.6 and 2.3, under the fit's limit of 6), so both still
refuse to open. The R6 Mark III kept only its D55 matrix: its one sample's
fit read its own Tungsten preset as 4146 K. And the RP and R50 spectral rows
sat 6–11 ΔE from their camera JPEGs, against 1–3 ΔE for the other spectral
Canon bodies, which raised doubt about their data.

## Owner decisions (2026-11-24)

- **Samples.** Owner-supplied CR3s (`~/Downloads/PHOTOSHOP/RAW/Canon`; more
  models in the dpreview sample galleries). They are fit inputs only and are
  not committed; `camera-jpeg-fits.json` records each file's name and
  SHA-256.
- **RP / R50.** Investigate first. Switch a body to its camera-JPEG fit only
  if its spectral row proves unsound; otherwise keep it and record why.
- **Scope.** The R3, the V1, the R6 Mark III pair and the RP / R50 rows; any
  other body only if a usable sample turns up. The rest goes to a follow-up
  issue, and the change closes #1579.

## What changes

- `scripts/fit-camera-jpeg-matrices.py` knows colour-data version 67 (the R8
  Mark II's, laid out like 64).
- `scripts/gen-camera-matrices.py` takes, of a body's several accepted
  samples, the first with both matrices (else the first), so the R6 Mark
  III's new sample restores its pair.
- Fits from the owner's samples add rows for the EOS R3, PowerShot V1 and EOS
  R1, and give the R6 Mark III its D55 + Tungsten pair. No other row
  changes.
- The RP and R50 spectral rows stay (see design: Canon's own presets
  support both, and the R50 shares the R10's sensor response).

## Capabilities

- Modified: `codec/camera-matrices`.
