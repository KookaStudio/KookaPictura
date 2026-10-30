//! Image ▸ Adjustments ▸ Gradient Map: the dialog edits a `grdm` block through
//! the Gradient Editor. Stops cross the bridge as `"location:RRGGBB[AA] …"`,
//! `location` in the PSD's `0..=4096` and the optional `AA` the stop's opacity
//! (`ff` when absent). The editor's Smoothness and its Noise gradients are baked
//! into plain stops here, since a `grdm` block carries only stops. Free
//! functions (their own bridge, so the `PictureView` declaration list does not
//! grow).

mod imp;

use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// A Gradient Map `block`'s stops, or empty for another block. Each
        /// carries the opacity at its location; an opacity stop between colour
        /// stops gets a colour stop of its own.
        fn gradient_map_stops(block: &[u8]) -> QString;

        /// A Gradient Map `block`'s Reverse flag (false for another block).
        fn gradient_map_reverse(block: &[u8]) -> bool;

        /// A Gradient Map `block`'s Dither flag (false for another block).
        fn gradient_map_dither(block: &[u8]) -> bool;

        /// The Gradient Map block of `stops` (nudged apart so their locations
        /// strictly increase) at `smoothness` 0–100 %, or empty for fewer than
        /// two stops or a bad list. Stops below full opacity add opacity stops.
        fn gradient_map_block(
            stops: &QString,
            smoothness: i32,
            reverse: bool,
            dither: bool,
        ) -> Vec<u8>;

        /// `stops` with `smoothness` baked in, as the ramp to draw.
        fn gradient_smoothed_stops(stops: &QString, smoothness: i32) -> QString;

        /// A Noise gradient: `seed` picks it, `roughness` 0–100 % sets how often
        /// it changes, `model` (0 RGB, 1 HSB, 2 Lab) names the components whose
        /// ranges `low0..high0` … `low2..high2` (percent of each) bound the
        /// colours. `restrict` keeps them from oversaturating; `transparency`
        /// gives the stops random opacity.
        #[allow(clippy::too_many_arguments)]
        fn gradient_noise_stops(
            seed: u32,
            roughness: i32,
            model: i32,
            low0: i32,
            high0: i32,
            low1: i32,
            high1: i32,
            low2: i32,
            high2: i32,
            restrict: bool,
            transparency: bool,
        ) -> QString;

        /// Built-in gradient `index` (the Gradient tool's presets) between
        /// `foreground` and `background` (`0xAARRGGBB`) as stops, locations
        /// strictly increasing. Empty out of range.
        fn gradient_preset_map_stops(index: i32, foreground: u32, background: u32) -> QString;
    }
}
