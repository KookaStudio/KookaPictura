//! Write `pictura_color::Profile::adobe_rgb()`'s ICC bytes to stdout.
//!
//! Regenerates the `psd_icc_rgb.icc` test fixture without bundling an Adobe
//! profile file:
//!
//! ```sh
//! cargo run -q -p pictura-color --example dump_adobe_rgb \
//!   > crates/pictura-codec/tests/fixtures/psd_icc_rgb.icc
//! ```

use std::io::Write;

fn main() {
    let bytes = pictura_color::Profile::adobe_rgb().to_icc();
    std::io::stdout().write_all(&bytes).expect("write ICC");
}
