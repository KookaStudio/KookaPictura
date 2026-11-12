//! R7 of #1217: LZ4 (`lz4_flex`) against deflate (`flate2`) for packing tiles,
//! measured on the dense tiles of the M12 fixture.
//!
//! ```text
//! python3 scripts/gen-large-fixtures.py --size 12000 --out ~/m12.psd
//! PICTURA_PACK_BENCH_PSD=~/m12.psd cargo test --release -p pictura-codec \
//!     --test pack_codec_benchmark -- --ignored --nocapture
//! ```

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use pictura_core::TileData;

/// The dense tiles of every layer channel, as bytes.
fn dense_tiles(path: &str) -> Vec<Vec<u8>> {
    let bytes = std::fs::read(path).expect("the M12 fixture");
    let doc = pictura_codec::read_psd(&bytes).expect("a readable PSD");
    let mut tiles = Vec::new();
    for layer in &doc.layers {
        for channel in &layer.channels {
            for i in 0..channel.data.tile_count() {
                if let Some(tile) = channel.data.tile_at(i) {
                    if let TileData::Dense(d) = &tile.data {
                        tiles.push(d.to_vec());
                    }
                }
            }
        }
    }
    tiles
}

struct Result {
    name: &'static str,
    packed: usize,
    pack: Duration,
    unpack: Duration,
}

fn measure(
    name: &'static str,
    tiles: &[Vec<u8>],
    pack: impl Fn(&[u8]) -> Vec<u8>,
    unpack: impl Fn(&[u8], usize) -> Vec<u8>,
) -> Result {
    let start = Instant::now();
    let packed: Vec<Vec<u8>> = tiles.iter().map(|t| pack(t)).collect();
    let pack_time = start.elapsed();
    let start = Instant::now();
    for (p, t) in packed.iter().zip(tiles) {
        assert_eq!(&unpack(p, t.len()), t, "{name} round-trips");
    }
    Result {
        name,
        packed: packed.iter().map(Vec::len).sum(),
        pack: pack_time,
        unpack: start.elapsed(),
    }
}

#[test]
#[ignore = "benchmark: needs PICTURA_PACK_BENCH_PSD and a release build"]
fn pack_codec_benchmark() {
    let Ok(path) = std::env::var("PICTURA_PACK_BENCH_PSD") else {
        eprintln!("PICTURA_PACK_BENCH_PSD is not set; skipping");
        return;
    };
    let tiles = dense_tiles(&path);
    let raw: usize = tiles.iter().map(Vec::len).sum();
    let mib = raw as f64 / (1024.0 * 1024.0);
    let results = [
        measure("lz4_flex", &tiles, lz4_flex::block::compress, |p, n| {
            lz4_flex::block::decompress(p, n).unwrap()
        }),
        measure(
            "flate2 fast",
            &tiles,
            |t| {
                let mut e =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
                e.write_all(t).unwrap();
                e.finish().unwrap()
            },
            |p, n| {
                let mut out = Vec::with_capacity(n);
                flate2::read::DeflateDecoder::new(p)
                    .read_to_end(&mut out)
                    .unwrap();
                out
            },
        ),
    ];
    println!("{} dense tiles, {mib:.0} MiB", tiles.len());
    for r in &results {
        println!(
            "{:12} ratio {:5.3}  pack {:7.0} MiB/s  unpack {:7.0} MiB/s",
            r.name,
            r.packed as f64 / raw as f64,
            mib / r.pack.as_secs_f64(),
            mib / r.unpack.as_secs_f64(),
        );
    }
}
