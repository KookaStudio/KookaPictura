//! Version-16 `.atn` action-set and `Actions Palette.psp` container codec.
//!
//! Files carry no leading magic, all container integers are big-endian, and the
//! embedded descriptor bodies have no version or length prefix. Names in the
//! container are counted UTF-16BE code units; event identifiers and display
//! names are length-prefixed bytes and are preserved verbatim.

use crate::action_descriptor::{
    bounded_count, error, read_action_descriptor_from, read_bytes, read_utf16_units, take,
    take_i32, take_u16, take_u32, take_u8, write_action_descriptor_to, ActionDescriptor, AtnError,
};
use crate::common::Reader;

const VERSION: u32 = 16;

/// A step's event identifier. `Long` keeps the raw four-character code and
/// `Text` keeps the exact byte string, so unknown events round-trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawEventId {
    Text(Vec<u8>),
    Long([u8; 4]),
}

/// One raw recorded step.
///
/// Flag bytes accept any nonzero value on read and are written as `0`/`1`, so a
/// non-canonical flag is normalised to its semantic value.
#[derive(Debug, Clone, PartialEq)]
pub struct RawStep {
    pub expanded: bool,
    pub enabled: bool,
    pub with_dialog: bool,
    /// Dialog options; value 4 is accepted on read and written as 0.
    pub dialog_options: u8,
    pub event_id: RawEventId,
    pub display_name: Vec<u8>,
    pub descriptor: Option<ActionDescriptor>,
}

/// One raw action.
///
/// The Shift, Ctrl, and expanded flags follow [`RawStep`]'s byte
/// normalisation.
#[derive(Debug, Clone, PartialEq)]
pub struct RawAction {
    pub function_key: u16,
    pub shift: bool,
    pub ctrl: bool,
    pub color: u16,
    pub name: Vec<u16>,
    pub expanded: bool,
    pub steps: Vec<RawStep>,
}

/// One raw action set.
///
/// The expanded flag follows [`RawStep`]'s byte normalisation.
#[derive(Debug, Clone, PartialEq)]
pub struct RawActionSet {
    pub name: Vec<u16>,
    pub expanded: bool,
    pub actions: Vec<RawAction>,
}

/// A complete `.atn` file: exactly one set plus any trailing bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct RawAtn {
    pub set: RawActionSet,
    pub trailing: Vec<u8>,
}

/// Read a version-16 `.atn` file.
pub fn read_atn(bytes: &[u8]) -> Result<RawAtn, AtnError> {
    let mut r = Reader::new(bytes);
    read_version(&mut r, "action file")?;
    let set = read_set(&mut r)?;
    let trailing = r.data[r.pos..].to_vec();
    Ok(RawAtn { set, trailing })
}

/// Write a version-16 `.atn` file, re-emitting trailing bytes unchanged.
pub fn write_atn(value: &RawAtn) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&VERSION.to_be_bytes());
    write_set(&value.set, &mut out);
    out.extend_from_slice(&value.trailing);
    out
}

/// Read a version-16 actions palette. A malformed set returns the sets decoded
/// before it in [`AtnError::partial_sets`]; trailing bytes are rejected.
pub fn read_psp(bytes: &[u8]) -> Result<Vec<RawActionSet>, AtnError> {
    let mut r = Reader::new(bytes);
    read_version(&mut r, "actions palette")?;
    let count = take_u32(&mut r)?;
    bounded_count(&r, count, 9, "palette set")?;
    let mut sets = Vec::new();
    for index in 0..count {
        match read_set(&mut r) {
            Ok(set) => sets.push(set),
            Err(mut failure) => {
                failure.partial_sets = sets;
                failure.message = format!(
                    "actions palette set {index} of {count}: {}",
                    failure.message
                );
                return Err(failure);
            }
        }
    }
    if r.pos != bytes.len() {
        let mut failure = error(
            format!("actions palette has {} trailing bytes", bytes.len() - r.pos),
            r.pos,
        );
        failure.partial_sets = sets;
        return Err(failure);
    }
    Ok(sets)
}

/// Write a version-16 actions palette.
pub fn write_psp(sets: &[RawActionSet]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&VERSION.to_be_bytes());
    out.extend_from_slice(&(sets.len() as u32).to_be_bytes());
    for set in sets {
        write_set(set, &mut out);
    }
    out
}

fn read_version(r: &mut Reader, kind: &str) -> Result<(), AtnError> {
    let at = r.pos;
    let version = take_u32(r)?;
    if version != VERSION {
        return Err(error(format!("unsupported {kind} version {version}"), at));
    }
    Ok(())
}

fn read_set(r: &mut Reader) -> Result<RawActionSet, AtnError> {
    let name = read_utf16_units(r)?;
    let expanded = take_u8(r)? != 0;
    let count = take_u32(r)?;
    let n = bounded_count(r, count, 1, "action")?;
    let mut actions = Vec::new();
    for _ in 0..n {
        actions.push(read_action(r)?);
    }
    Ok(RawActionSet {
        name,
        expanded,
        actions,
    })
}

fn read_action(r: &mut Reader) -> Result<RawAction, AtnError> {
    let function_key = take_u16(r)?;
    let shift = take_u8(r)? != 0;
    let ctrl = take_u8(r)? != 0;
    let color = take_u16(r)?;
    let name = read_utf16_units(r)?;
    let expanded = take_u8(r)? != 0;
    let count = take_u32(r)?;
    let n = bounded_count(r, count, 1, "step")?;
    let mut steps = Vec::new();
    for _ in 0..n {
        steps.push(read_step(r)?);
    }
    Ok(RawAction {
        function_key,
        shift,
        ctrl,
        color,
        name,
        expanded,
        steps,
    })
}

fn read_step(r: &mut Reader) -> Result<RawStep, AtnError> {
    let expanded = take_u8(r)? != 0;
    let enabled = take_u8(r)? != 0;
    let with_dialog = take_u8(r)? != 0;
    let dialog_options = take_u8(r)?;
    let format_at = r.pos;
    let format = take(r, 4)?;
    let event_id = match format {
        b"long" => {
            let code = take(r, 4)?;
            RawEventId::Long([code[0], code[1], code[2], code[3]])
        }
        b"TEXT" => RawEventId::Text(read_bytes(r, "event identifier")?),
        _ => {
            return Err(error(
                format!(
                    "unknown event-id format {:?}",
                    String::from_utf8_lossy(format)
                ),
                format_at,
            ))
        }
    };
    let display_name = read_bytes(r, "event display name")?;
    let marker_at = r.pos;
    let marker = take_i32(r)?;
    let descriptor = match marker {
        0 => None,
        -1 => Some(read_action_descriptor_from(r, true)?),
        _ => {
            return Err(error(
                format!("invalid has-descriptor marker {marker}"),
                marker_at,
            ))
        }
    };
    Ok(RawStep {
        expanded,
        enabled,
        with_dialog,
        dialog_options,
        event_id,
        display_name,
        descriptor,
    })
}

fn write_set(set: &RawActionSet, out: &mut Vec<u8>) {
    write_utf16_units(&set.name, out);
    out.push(u8::from(set.expanded));
    out.extend_from_slice(&(set.actions.len() as u32).to_be_bytes());
    for action in &set.actions {
        write_action(action, out);
    }
}

fn write_action(action: &RawAction, out: &mut Vec<u8>) {
    out.extend_from_slice(&action.function_key.to_be_bytes());
    out.push(u8::from(action.shift));
    out.push(u8::from(action.ctrl));
    out.extend_from_slice(&action.color.to_be_bytes());
    write_utf16_units(&action.name, out);
    out.push(u8::from(action.expanded));
    out.extend_from_slice(&(action.steps.len() as u32).to_be_bytes());
    for step in &action.steps {
        write_step(step, out);
    }
}

fn write_step(step: &RawStep, out: &mut Vec<u8>) {
    out.push(u8::from(step.expanded));
    out.push(u8::from(step.enabled));
    out.push(u8::from(step.with_dialog));
    out.push(if step.dialog_options == 4 {
        0
    } else {
        step.dialog_options
    });
    match &step.event_id {
        RawEventId::Long(code) => {
            out.extend_from_slice(b"long");
            out.extend_from_slice(code);
        }
        RawEventId::Text(bytes) => {
            out.extend_from_slice(b"TEXT");
            write_bytes(bytes, out);
        }
    }
    write_bytes(&step.display_name, out);
    match &step.descriptor {
        Some(descriptor) => {
            out.extend_from_slice(&(-1i32).to_be_bytes());
            write_action_descriptor_to(descriptor, true, out);
        }
        None => out.extend_from_slice(&0i32.to_be_bytes()),
    }
}

fn write_utf16_units(units: &[u16], out: &mut Vec<u8>) {
    out.extend_from_slice(&(units.len() as u32).to_be_bytes());
    for unit in units {
        out.extend_from_slice(&unit.to_be_bytes());
    }
}

fn write_bytes(bytes: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(bytes);
}
