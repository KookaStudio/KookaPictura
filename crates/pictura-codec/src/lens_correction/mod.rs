//! Lens Correction smart filter: typed read/write of the dialog's 16 slots.
//!
//! Pictura-only (see the change design): the descriptor's `filterID`, class id,
//! name, and short keys are ours, not the reference Photoshop filter's, so a
//! document authored here re-opens here and no cross-app interop is claimed.
//! Keys this codec does not model are preserved by the generic smart-filter
//! attach.

use pictura_core::{Layer, SmartFilter};

use crate::descriptor::{self, get_object_item, DescValue};

#[cfg(test)]
mod tests;

/// The PSD smart-filter id of the Lens Correction filter (Pictura-only; it does
/// not collide with the reference camera-raw id 2683).
pub const LENS_CORRECTION_FILTER_ID: i32 = 0x5054_4C43; // "PTLC"
/// The filter's display name as stored in `filterFXList[].Nm  `.
pub const LENS_CORRECTION_FILTER_NAME: &str = "Lens Correction";

/// The opening slot values, in `LensCorrectionParams::SLOTS` order.
pub const DEFAULT_SLOTS: [f64; 16] = [
    1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 50.0, 0.0, 0.0, 0.0, 100.0,
];

/// Each slot's descriptor key, in order. Slots 0..=3 are booleans, 4 and 5 are
/// integers, and the rest are doubles.
const SLOT_KEYS: [&[u8]; 16] = [
    b"Dstn", b"CAOn", b"VgnO", b"AScl", b"Edge", b"Prof", b"RDis", b"FRCY", b"FGMG", b"FBYL",
    b"VigA", b"VigM", b"PerV", b"PerH", b"Angl", b"Scal",
];

/// The dialog's slots as a `Fltr` descriptor object.
pub(crate) fn encode_lens_correction_fltr(slots: &[f64]) -> DescValue {
    let slot = |i: usize| {
        slots
            .get(i)
            .copied()
            .filter(|v| v.is_finite())
            .unwrap_or(DEFAULT_SLOTS[i])
    };
    let items = SLOT_KEYS
        .iter()
        .enumerate()
        .map(|(i, key)| {
            let value = match i {
                0..=3 => DescValue::Bool(slot(i) >= 0.5),
                4 | 5 => DescValue::Long(slot(i).round() as i32),
                _ => DescValue::Double(slot(i)),
            };
            (key.to_vec(), value)
        })
        .collect();
    DescValue::Object {
        name: String::new(),
        class_id: b"Pictura Lens Correction Filter".to_vec(),
        items,
    }
}

/// The dialog's slots from a filter options buffer, each defaulting to its
/// opening value when the key is missing or malformed.
pub fn decode_lens_correction_slots(options: &[u8]) -> Vec<f64> {
    let mut slots = DEFAULT_SLOTS;
    let Ok(DescValue::Object { items, .. }) = crate::read_descriptor(options) else {
        return slots.to_vec();
    };
    for (i, key) in SLOT_KEYS.iter().enumerate() {
        let number = match get_object_item(&items, key) {
            Some(DescValue::Bool(v)) => Some(if *v { 1.0 } else { 0.0 }),
            Some(DescValue::Long(v)) => Some(*v as f64),
            Some(DescValue::Double(v)) if v.is_finite() => Some(*v),
            _ => None,
        };
        if let Some(v) = number {
            slots[i] = v;
        }
    }
    slots.to_vec()
}

/// Attach a Lens Correction smart filter carrying `slots` to `layer`,
/// replacing an existing Lens Correction entry or appending a new one.
pub fn attach_lens_correction_filter(
    layer: &mut Layer,
    slots: &[f64],
) -> Result<(), crate::PsdError> {
    let options = descriptor::write_descriptor(&encode_lens_correction_fltr(slots));
    crate::attach_smart_filter(
        layer,
        SmartFilter {
            filter_id: LENS_CORRECTION_FILTER_ID,
            name: LENS_CORRECTION_FILTER_NAME.to_string(),
            enabled: true,
            options,
        },
    )
}
