//! 256-entry adjustment lookup tables uploaded for the GPU.

use pictura_adjust::AdjustmentLut;

/// The shader's adjustment kind for a table.
pub(super) fn kind(lut: &AdjustmentLut) -> u32 {
    match lut {
        AdjustmentLut::PerChannel(_) => 6,
        AdjustmentLut::Luma(_) => 7,
    }
}

/// The table as 256 `vec4<f32>` entries (4096 bytes).
#[allow(clippy::needless_range_loop)]
pub(super) fn bytes(lut: &AdjustmentLut) -> Vec<u8> {
    let mut out = Vec::with_capacity(256 * 16);
    match lut {
        AdjustmentLut::PerChannel(table) => {
            for i in 0..256 {
                let entry = [
                    table[0][i] as f32 / 255.0,
                    table[1][i] as f32 / 255.0,
                    table[2][i] as f32 / 255.0,
                    0.0,
                ];
                for v in entry {
                    out.extend_from_slice(&v.to_le_bytes());
                }
            }
        }
        AdjustmentLut::Luma(table) => {
            for entry in table {
                for v in [
                    entry[0] as f32 / 255.0,
                    entry[1] as f32 / 255.0,
                    entry[2] as f32 / 255.0,
                    entry[3] as f32 / 255.0,
                ] {
                    out.extend_from_slice(&v.to_le_bytes());
                }
            }
        }
    }
    out
}
