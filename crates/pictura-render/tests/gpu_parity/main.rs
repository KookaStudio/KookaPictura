//! GPU vs CPU parity for the separable blend modes.
//!
//! The CPU compositor is the oracle. Each scene drives one blend mode with a
//! gradient backdrop and a partially transparent gradient source, and the GPU
//! output must match `composite_rgba` within ±1 LSB per channel.
//!
//! A test that needs a device is `#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]` and
//! fails when run without one, so a skipped parity check reports as skipped,
//! never as passed. CI runs them on lavapipe and WARP (`software-gpu`, the
//! nextest `gpu` profile); locally, run the same command on any Vulkan machine.
//!
//! M31 adds region compositing: a region-limited GPU composite must be
//! byte-identical to the same slice of the full `composite_active` result,
//! because it is the same per-pixel kernel over a region-sized canvas.

mod blend;
mod common;
mod fresh_white;
mod large;
mod layers;
mod region;
mod text;
