//! Portable Photoshop print settings in the version-16 image resources.

use pictura_color::Intent;
use pictura_core::Document;

use crate::action_descriptor::{
    read_action_descriptor, write_action_descriptor, ActionClass, ActionDescriptor, ActionKey,
    ActionValue,
};
use crate::image_resources::{
    decode_section, encode_image_resources, frame_image_resource, ImageResource,
};
use crate::PsdError;

#[cfg(test)]
mod tests;

const PRINT_FLAGS: u16 = 1011;
const PRINT_INFO: u16 = 1082;
const PRINT_STYLE: u16 = 1083;
const EXTENDED_FLAGS: u16 = 10000;

/// Accepted print resource changes that can be applied to a later save
/// snapshot without replacing metadata owned by other editors.
#[derive(Default)]
pub struct PrintResourceEdits(Vec<ImageResource>);

impl PrintResourceEdits {
    pub fn between(original: &Document, updated: &Document) -> Self {
        let (before, _) = decode_section(&original.image_resources);
        let (after, _) = decode_section(&updated.image_resources);
        let edits = [PRINT_INFO, PRINT_STYLE, PRINT_FLAGS, EXTENDED_FLAGS]
            .into_iter()
            .filter_map(|id| {
                let next = after.iter().find(|resource| resource.id == id)?;
                before
                    .iter()
                    .find(|old| old.id == id)
                    .is_none_or(|old| old.raw != next.raw)
                    .then(|| next.clone())
            })
            .collect();
        Self(edits)
    }

    pub fn apply(&self, doc: &mut Document) {
        if self.0.is_empty() {
            return;
        }
        let (mut resources, consumed) = decode_section(&doc.image_resources);
        for patch in &self.0 {
            if let Some(slot) = resources
                .iter_mut()
                .find(|resource| resource.id == patch.id)
            {
                *slot = patch.clone();
            } else {
                resources.push(patch.clone());
            }
        }
        let mut section = encode_image_resources(&resources);
        section.extend_from_slice(&doc.image_resources[consumed..]);
        doc.image_resources = section.into();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrintProofProfile {
    Builtin(String),
    Named(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrintProof {
    pub profile: PrintProofProfile,
    pub intent: Option<Intent>,
    pub black_point: Option<bool>,
    pub paper_white: Option<bool>,
}

/// Absent values leave the application's corresponding defaults in effect.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrintSettings {
    pub manage_colors: Option<bool>,
    pub printer_profile: Option<String>,
    pub printer_components: Option<u8>,
    pub intent: Option<Intent>,
    pub black_point: Option<bool>,
    pub hard_proof: Option<bool>,
    pub proof: Option<PrintProof>,
    pub calibration: Option<bool>,
    pub registration: Option<bool>,
    pub corner_crop: Option<bool>,
    pub center_crop: Option<bool>,
    pub filename: Option<bool>,
    pub description: Option<bool>,
    pub background: Option<[f64; 3]>,
    pub border: Option<f64>,
    pub bleed: Option<f64>,
}

fn key(bytes: &[u8]) -> ActionKey {
    if let Ok(code) = bytes.try_into() {
        ActionKey::Code(code)
    } else {
        ActionKey::String(bytes.to_vec())
    }
}

fn key_bytes(key: &ActionKey) -> &[u8] {
    match key {
        ActionKey::Code(code) => code,
        ActionKey::String(bytes) => bytes,
    }
}

fn descriptor(class: &[u8]) -> ActionDescriptor {
    ActionDescriptor {
        name: "\0".into(),
        class: ActionClass {
            name: "\0".into(),
            id: key(class),
        },
        items: Vec::new(),
    }
}

fn unpack(resource: &ImageResource, class: &[u8]) -> Result<ActionDescriptor, PsdError> {
    let invalid = || PsdError::Invalid(format!("cannot edit print resource {}", resource.id));
    if !resource.data.starts_with(&16u32.to_be_bytes()) {
        return Err(invalid());
    }
    let desc = read_action_descriptor(&resource.data[4..], false).map_err(|_| invalid())?;
    if key_bytes(&desc.class.id) != class {
        return Err(invalid());
    }
    Ok(desc)
}

fn pack(id: u16, name: &str, desc: &ActionDescriptor) -> ImageResource {
    let mut data = 16u32.to_be_bytes().to_vec();
    data.extend(write_action_descriptor(desc, false));
    frame_image_resource(id, name, &data)
}

fn value<'a>(desc: &'a ActionDescriptor, name: &[u8]) -> Option<&'a ActionValue> {
    desc.items
        .iter()
        .rev()
        .find(|(id, _)| key_bytes(id) == name)
        .map(|(_, value)| value)
}

fn set(desc: &mut ActionDescriptor, name: &[u8], value: Option<ActionValue>) {
    let mut value = value;
    desc.items.retain_mut(|(id, old)| {
        if key_bytes(id) != name {
            return true;
        }
        if let Some(next) = value.take() {
            *old = next;
            return true;
        }
        false
    });
    if let Some(value) = value {
        desc.items.push((key(name), value));
    }
}

fn boolean(desc: &ActionDescriptor, name: &[u8]) -> Option<bool> {
    match value(desc, name)? {
        ActionValue::Boolean(value) => Some(*value),
        _ => None,
    }
}

fn string(desc: &ActionDescriptor, name: &[u8]) -> Option<String> {
    match value(desc, name)? {
        ActionValue::Text(value) => {
            let value = value.trim_end_matches('\0');
            (!value.is_empty()).then(|| value.to_owned())
        }
        _ => None,
    }
}

fn text_value(mut value: String) -> ActionValue {
    value.push('\0');
    ActionValue::Text(value)
}

fn enumeration<'a>(desc: &'a ActionDescriptor, name: &[u8], ty: &[u8]) -> Option<&'a [u8]> {
    match value(desc, name)? {
        ActionValue::Enumerated { kind, value } if key_bytes(kind) == ty => Some(key_bytes(value)),
        _ => None,
    }
}

fn enum_value(ty: &[u8], id: &[u8]) -> ActionValue {
    ActionValue::Enumerated {
        kind: key(ty),
        value: key(id),
    }
}

fn intent(desc: &ActionDescriptor) -> Option<Intent> {
    match enumeration(desc, b"Inte", b"Inte")? {
        b"Img " => Some(Intent::Perceptual),
        b"Clrm" => Some(Intent::RelativeColorimetric),
        b"Grp " => Some(Intent::Saturation),
        b"AClr" => Some(Intent::AbsoluteColorimetric),
        _ => None,
    }
}

fn intent_value(intent: Intent) -> ActionValue {
    enum_value(
        b"Inte",
        match intent {
            Intent::Perceptual => b"Img ",
            Intent::RelativeColorimetric => b"Clrm",
            Intent::Saturation => b"Grp ",
            Intent::AbsoluteColorimetric => b"AClr",
        },
    )
}

fn unit_points(desc: &ActionDescriptor, name: &[u8]) -> Option<f64> {
    let ActionValue::UnitFloat { unit, value } = value(desc, name)? else {
        return None;
    };
    let points = value
        * match unit {
            b"#Rlt" | b"#Pnt" => 1.0,
            b"#In " => 72.0,
            b"#Mlm" => 72.0 / 25.4,
            b"#Cmt" => 72.0 / 2.54,
            _ => return None,
        };
    (points.is_finite() && points >= 0.0).then_some(points)
}

fn unit_value(points: f64) -> ActionValue {
    ActionValue::UnitFloat {
        unit: *b"#Rlt",
        value: points,
    }
}

fn nested<'a>(desc: &'a ActionDescriptor, name: &[u8]) -> Option<&'a ActionDescriptor> {
    match value(desc, name)? {
        ActionValue::Descriptor(nested) => Some(nested),
        _ => None,
    }
}

fn read_proof(desc: &ActionDescriptor) -> Option<PrintProof> {
    let proof = nested(desc, b"printProofSetup")?;
    if key_bytes(&proof.class.id) != b"proofSetup" {
        return None;
    }
    let profile = if let Some(builtin) = enumeration(proof, b"Bltn", b"builtinProof") {
        PrintProofProfile::Builtin(std::str::from_utf8(builtin).ok()?.to_owned())
    } else {
        PrintProofProfile::Named(string(proof, b"profile")?)
    };
    Some(PrintProof {
        profile,
        intent: intent(proof),
        black_point: boolean(proof, b"MpBl"),
        paper_white: boolean(proof, b"paperWhite"),
    })
}

fn write_proof(desc: &ActionDescriptor, proof: &PrintProof) -> ActionValue {
    let mut result = nested(desc, b"printProofSetup")
        .cloned()
        .unwrap_or_else(|| {
            let mut proof = descriptor(b"proofSetup");
            proof.name = "Proof Setup\0".into();
            proof
        });
    match &proof.profile {
        PrintProofProfile::Builtin(name) => {
            set(
                &mut result,
                b"Bltn",
                Some(enum_value(b"builtinProof", name.as_bytes())),
            );
            set(&mut result, b"profile", None);
        }
        PrintProofProfile::Named(name) => {
            set(&mut result, b"Bltn", None);
            set(&mut result, b"profile", Some(text_value(name.clone())));
        }
    }
    set(&mut result, b"Inte", proof.intent.map(intent_value));
    set(
        &mut result,
        b"MpBl",
        proof.black_point.map(ActionValue::Boolean),
    );
    set(
        &mut result,
        b"paperWhite",
        proof.paper_white.map(ActionValue::Boolean),
    );
    ActionValue::Descriptor(Box::new(result))
}

fn background(desc: &ActionDescriptor) -> Option<[f64; 3]> {
    let color = nested(desc, b"Bckg")?;
    let channel = |name: &[u8]| match value(color, name)? {
        ActionValue::Double(n) if (0.0..=255.0).contains(n) => Some(*n),
        _ => None,
    };
    Some([channel(b"Rd  ")?, channel(b"Grn ")?, channel(b"Bl  ")?])
}

fn background_value(desc: &ActionDescriptor, rgb: [f64; 3]) -> ActionValue {
    let mut color = nested(desc, b"Bckg")
        .cloned()
        .unwrap_or_else(|| descriptor(b"RGBC"));
    for (name, channel) in [b"Rd  ", b"Grn ", b"Bl  "].into_iter().zip(rgb) {
        set(&mut color, name, Some(ActionValue::Double(channel)));
    }
    ActionValue::Descriptor(Box::new(color))
}

pub fn read_print_settings(doc: &Document) -> PrintSettings {
    let (resources, _) = decode_section(&doc.image_resources);
    let find = |id| resources.iter().find(|resource| resource.id == id);
    let flags = find(PRINT_FLAGS)
        .map(|resource| resource.data.as_slice())
        .unwrap_or_default();
    let legacy = |index| flags.get(index).map(|flag| *flag != 0);
    let mut settings = PrintSettings {
        filename: legacy(0),
        corner_crop: legacy(1),
        calibration: legacy(2),
        registration: legacy(3),
        description: legacy(7),
        center_crop: find(EXTENDED_FLAGS)
            .filter(|r| r.data.starts_with(&[0, 1]))
            .and_then(|r| r.data.get(2))
            .map(|flag| *flag != 0),
        ..PrintSettings::default()
    };
    if let Some(info) = find(PRINT_INFO).and_then(|r| unpack(r, b"printOutput").ok()) {
        settings.manage_colors = Some(!boolean(&info, b"PstS").unwrap_or(false));
        settings.printer_profile = string(&info, b"Nm  ");
        settings.printer_components = match enumeration(&info, b"ClrS", b"ClrS") {
            Some(b"RGBC") => Some(3),
            Some(b"Grys") => Some(1),
            _ => None,
        };
        settings.intent = intent(&info);
        settings.black_point = boolean(&info, b"MpBl");
        settings.hard_proof = boolean(&info, b"hardProof");
        settings.proof = read_proof(&info);
    }
    if let Some(style) = find(PRINT_STYLE).and_then(|r| unpack(r, b"printOutputOptions").ok()) {
        settings.calibration = boolean(&style, b"Clbr").or(settings.calibration);
        settings.registration = boolean(&style, b"RgsM").or(settings.registration);
        settings.corner_crop = boolean(&style, b"CrnC").or(settings.corner_crop);
        settings.center_crop = boolean(&style, b"CntC").or(settings.center_crop);
        settings.filename = boolean(&style, b"Lbls").or(settings.filename);
        settings.description = boolean(&style, b"Cptn").or(settings.description);
        settings.background = background(&style);
        settings.border = unit_points(&style, b"BrdT");
        settings.bleed = unit_points(&style, b"Bld ");
    }
    settings
}

fn edit_resource(
    resources: &mut Vec<ImageResource>,
    id: u16,
    class: &[u8],
    edit: impl FnOnce(&mut ActionDescriptor),
) -> Result<(), PsdError> {
    if resources
        .iter()
        .filter(|resource| resource.id == id)
        .count()
        > 1
    {
        return Err(PsdError::Invalid(format!("ambiguous print resource {id}")));
    }
    if let Some(resource) = resources.iter_mut().find(|resource| resource.id == id) {
        let mut desc = unpack(resource, class)?;
        edit(&mut desc);
        *resource = pack(id, &resource.name, &desc);
    } else {
        let mut desc = descriptor(class);
        edit(&mut desc);
        resources.push(pack(id, "", &desc));
    }
    Ok(())
}

/// Apply only changed print fields to an existing resource section.
pub fn update_print_settings(
    doc: &mut Document,
    old: &PrintSettings,
    new: &PrintSettings,
) -> Result<bool, PsdError> {
    if old == new {
        return Ok(false);
    }
    let (mut resources, consumed) = decode_section(&doc.image_resources);
    if consumed != doc.image_resources.len() {
        return Err(PsdError::Invalid(
            "incomplete image resource section".into(),
        ));
    }
    let mut info_changes = Vec::new();
    let mut style_changes = Vec::new();
    macro_rules! add_change {
        ($changes:ident, $field:ident, $name:literal, $convert:expr) => {
            if old.$field != new.$field {
                $changes.push(($name.as_slice(), new.$field.clone().map($convert)));
            }
        };
    }
    add_change!(info_changes, printer_profile, b"Nm  ", text_value);
    add_change!(info_changes, intent, b"Inte", intent_value);
    add_change!(info_changes, black_point, b"MpBl", ActionValue::Boolean);
    add_change!(info_changes, hard_proof, b"hardProof", ActionValue::Boolean);
    if old.manage_colors != new.manage_colors {
        info_changes.push((
            b"PstS",
            (new.manage_colors == Some(false)).then_some(ActionValue::Boolean(true)),
        ));
    }
    if old.printer_components != new.printer_components {
        // The public mapping supplies only the RGB and gray selectors. The
        // profile description is still stored for a CMYK output.
        let space = match new.printer_components {
            Some(1) => Some(enum_value(b"ClrS", b"Grys")),
            Some(3) => Some(enum_value(b"ClrS", b"RGBC")),
            _ => None,
        };
        info_changes.push((b"ClrS", space));
    }
    if !info_changes.is_empty() || old.proof != new.proof {
        edit_resource(&mut resources, PRINT_INFO, b"printOutput", |desc| {
            for (name, value) in info_changes {
                set(desc, name, value);
            }
            if old.proof != new.proof {
                let proof = new.proof.as_ref().map(|proof| write_proof(desc, proof));
                set(desc, b"printProofSetup", proof);
            }
            for (name, default) in [
                (
                    b"Inte".as_slice(),
                    intent_value(Intent::RelativeColorimetric),
                ),
                (b"printerName".as_slice(), text_value(String::new())),
                (b"printSixteenBit".as_slice(), ActionValue::Boolean(false)),
            ] {
                if value(desc, name).is_none() {
                    set(desc, name, Some(default));
                }
            }
        })?;
    }
    add_change!(style_changes, calibration, b"Clbr", ActionValue::Boolean);
    add_change!(style_changes, registration, b"RgsM", ActionValue::Boolean);
    add_change!(style_changes, corner_crop, b"CrnC", ActionValue::Boolean);
    add_change!(style_changes, center_crop, b"CntC", ActionValue::Boolean);
    add_change!(style_changes, filename, b"Lbls", ActionValue::Boolean);
    add_change!(style_changes, description, b"Cptn", ActionValue::Boolean);
    add_change!(style_changes, border, b"BrdT", unit_value);
    add_change!(style_changes, bleed, b"Bld ", unit_value);
    if !style_changes.is_empty() || old.background != new.background {
        edit_resource(&mut resources, PRINT_STYLE, b"printOutputOptions", |desc| {
            for (name, value) in style_changes {
                set(desc, name, value);
            }
            if old.background != new.background {
                let color = new.background.map(|color| background_value(desc, color));
                set(desc, b"Bckg", color);
            }
        })?;
    }
    for id in [PRINT_FLAGS, EXTENDED_FLAGS] {
        let Some(resource) = resources.iter_mut().find(|resource| resource.id == id) else {
            continue;
        };
        let mut flags = resource.data.clone();
        let mut apply = |index: usize, previous: Option<bool>, next: Option<bool>| {
            if let Some((flag, selected)) =
                flags.get_mut(index).zip(next.filter(|_| previous != next))
            {
                *flag = u8::from(selected);
            }
        };
        match resource.id {
            PRINT_FLAGS => {
                apply(0, old.filename, new.filename);
                apply(1, old.corner_crop, new.corner_crop);
                apply(2, old.calibration, new.calibration);
                apply(3, old.registration, new.registration);
                apply(7, old.description, new.description);
            }
            EXTENDED_FLAGS if resource.data.starts_with(&[0, 1]) => {
                apply(2, old.center_crop, new.center_crop);
            }
            _ => continue,
        }
        if flags != resource.data {
            *resource = frame_image_resource(resource.id, &resource.name, &flags);
        }
    }
    doc.image_resources = encode_image_resources(&resources).into();
    Ok(true)
}
