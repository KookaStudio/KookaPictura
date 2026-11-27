// Each RAW suite declares the shared synthetic-file builders with a relative
// `#[path]`, so the same helper file enters this binary once per suite.
#![allow(clippy::duplicate_mod)]

mod dng_camera_colour;
mod dng_demosaic;
mod dng_linear;
mod dng_reader;
mod dng_writer_oracle;
mod raw_arw;
mod raw_camera_matrices;
mod raw_cr2;
mod raw_cr3;
mod raw_cr3_colour;
mod raw_develop;
mod raw_nef;
mod raw_orf;
mod raw_raf;
mod raw_raf_colour;
mod raw_raf_compressed;
mod raw_raf_superccd;
mod raw_rw2;
mod raw_rw2_codings;
mod raw_rw2_colour;
