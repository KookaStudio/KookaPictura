//! The memory gates of the geometry commands: Image Size, Canvas Size, the
//! turns and flips, and arbitrary Rotate (#1293, #1294).

use pictura_core::Document;

use super::memory_gate;

/// Whether an Image Size of `doc` to `width` × `height` passes its gate
/// (#1293): each side within `ImageBudget`'s limit, then its commit cost
/// through [`memory_gate::admit_spilling`], which records a refusal. No plane
/// is allocated whole, so the per-allocation cap does not apply.
pub(in crate::cxxqt_object) fn image_size_admits(doc: &Document, width: i32, height: i32) -> bool {
    memory_gate::clear();
    if !sides_fit(i64::from(width), i64::from(height)) {
        return false;
    }
    let cost = memory_gate::image_size_cost(doc, width as u32, height as u32);
    memory_gate::admit_spilling(cost.working, cost.spillable)
}

/// Whether a Canvas Size of `doc` to `width` × `height` at `anchor` passes its
/// gate (#1294), as [`image_size_admits`] does for Image Size.
pub(in crate::cxxqt_object) fn canvas_size_admits(
    doc: &Document,
    width: i32,
    height: i32,
    anchor: pictura_ops::Anchor,
) -> bool {
    memory_gate::clear();
    if !sides_fit(i64::from(width), i64::from(height)) {
        return false;
    }
    let cost = memory_gate::canvas_size_cost(doc, width as u32, height as u32, anchor);
    memory_gate::admit_spilling(cost.working, cost.spillable)
}

/// Whether a turn of `doc` by `quarter_turns`, or a flip at 0, passes its
/// gate (#1294). A turn keeps the sides, so only the bytes are judged.
pub(in crate::cxxqt_object) fn orient_admits(doc: &Document, quarter_turns: u8) -> bool {
    memory_gate::clear();
    let cost = memory_gate::orient_cost(doc, quarter_turns);
    memory_gate::admit_spilling(cost.working, cost.spillable)
}

/// Whether an arbitrary Rotate of `doc` by `angle_deg` about its centre passes
/// its gate (#1294): the grown canvas within `ImageBudget`'s side limit, then
/// its commit cost.
pub(in crate::cxxqt_object) fn rotate_admits(doc: &Document, angle_deg: f64) -> bool {
    memory_gate::clear();
    let (cost, (width, height)) = memory_gate::rotate_cost(doc, angle_deg);
    sides_fit(i64::from(width), i64::from(height))
        && memory_gate::admit_spilling(cost.working, cost.spillable)
}

/// Whether both sides are within `ImageBudget`'s dimension limit.
fn sides_fit(width: i64, height: i64) -> bool {
    let max = i64::from(pictura_codec::ImageBudget::default().max_dimension);
    (1..=max).contains(&width) && (1..=max).contains(&height)
}
