//! The generic Pictura smart filter: any catalogue filter, or a Filter Gallery
//! stack, stored as its kinds and full-resolution slots.
//!
//! Pictura-only (see the smart-filter-apply design): the `filterID`, class id,
//! and short keys are ours, so a document authored here re-opens here and no
//! cross-app interop is claimed.

use crate::descriptor::{self, get_object_item, DescValue};

#[cfg(test)]
mod tests;

/// The PSD smart-filter id of the generic record (Pictura-only).
pub const PICTURA_FILTER_ID: i32 = 0x5054_464C; // "PTFL"
/// The entry name of a Filter Gallery stack.
pub const FILTER_GALLERY_NAME: &str = "Filter Gallery";

const VERSION: i32 = 1;

/// One filter of a record: a catalogue kind id and its slots.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterStep {
    pub kind: String,
    pub slots: Vec<f64>,
}

fn text(value: &str) -> DescValue {
    DescValue::Text(format!("{value}\0"))
}

/// The options bytes of a record holding `steps` in apply order; `gallery`
/// marks a Filter Gallery stack.
pub fn encode_pictura_filter(steps: &[FilterStep], gallery: bool) -> Vec<u8> {
    let stack = steps
        .iter()
        .map(|step| DescValue::Object {
            name: String::new(),
            class_id: b"Pictura Filter Step".to_vec(),
            items: vec![
                (b"Kind".to_vec(), text(&step.kind)),
                (
                    b"Prms".to_vec(),
                    DescValue::List(step.slots.iter().map(|v| DescValue::Double(*v)).collect()),
                ),
            ],
        })
        .collect();
    descriptor::write_descriptor(&DescValue::Object {
        name: String::new(),
        class_id: b"Pictura Filter".to_vec(),
        items: vec![
            (b"Vrsn".to_vec(), DescValue::Long(VERSION)),
            (b"Glry".to_vec(), DescValue::Bool(gallery)),
            (b"Stck".to_vec(), DescValue::List(stack)),
        ],
    })
}

/// The steps and gallery flag of a record, or `None` when the options are
/// malformed, of another version, or hold no step.
pub fn decode_pictura_filter(options: &[u8]) -> Option<(Vec<FilterStep>, bool)> {
    let DescValue::Object { items, .. } = crate::read_descriptor(options).ok()? else {
        return None;
    };
    if !matches!(
        get_object_item(&items, b"Vrsn"),
        Some(DescValue::Long(VERSION))
    ) {
        return None;
    }
    let Some(DescValue::Bool(gallery)) = get_object_item(&items, b"Glry") else {
        return None;
    };
    let Some(DescValue::List(stack)) = get_object_item(&items, b"Stck") else {
        return None;
    };
    let steps = stack
        .iter()
        .map(|entry| {
            let DescValue::Object { items, .. } = entry else {
                return None;
            };
            let Some(DescValue::Text(kind)) = get_object_item(items, b"Kind") else {
                return None;
            };
            let Some(DescValue::List(prms)) = get_object_item(items, b"Prms") else {
                return None;
            };
            let slots = prms
                .iter()
                .map(|v| match v {
                    DescValue::Double(v) if v.is_finite() => Some(*v),
                    _ => None,
                })
                .collect::<Option<Vec<f64>>>()?;
            Some(FilterStep {
                kind: kind.trim_end_matches('\0').to_string(),
                slots,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    (!steps.is_empty()).then_some((steps, *gallery))
}
