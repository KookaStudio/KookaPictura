use crate::action_descriptor::{
    read_action_descriptor, write_action_descriptor_to, ActionClass, ActionDescriptor, ActionKey,
    ActionReferenceItem, ActionValue, ReferenceForm, ReferenceValue,
};
use crate::image_resources::{
    decode_section, encode_image_resources, frame_image_resource, ImageResource,
};
use pictura_core::{Document, DocumentMeasurementScale, MeasurementScale};

#[cfg(test)]
mod tests;

const MEASUREMENT_SCALE_RESOURCE: u16 = 1074;
const VERSION: [u8; 4] = 16u32.to_be_bytes();
const SCALE_CLASS: &[u8] = b"measurementScale";
const SCALE_KEYS: [&[u8]; 3] = [b"pixelLength", b"logicalLength", b"logicalUnits"];

fn scale_resource(resources: &[ImageResource]) -> Option<&ImageResource> {
    resources
        .iter()
        .find(|resource| resource.id == MEASUREMENT_SCALE_RESOURCE)
}

fn is_key(key: &ActionKey, bytes: &[u8]) -> bool {
    match key {
        ActionKey::Code(code) => code.as_slice() == bytes,
        ActionKey::String(string) => string.as_slice() == bytes,
    }
}

fn scale_index(descriptor: &ActionDescriptor) -> Option<usize> {
    if !is_key(&descriptor.class.id, b"null") {
        return None;
    }
    let (_, ActionValue::Reference(target)) = descriptor
        .items
        .iter()
        .find(|(key, _)| is_key(key, b"null"))?
    else {
        return None;
    };
    let [property, document] = target.as_slice() else {
        return None;
    };
    let targets_scale = property.form == ReferenceForm::Property
        && is_key(&property.class.id, b"Prpr")
        && matches!(&property.value, ReferenceValue::Key(key) if is_key(key, SCALE_CLASS))
        && document.form == ReferenceForm::Enumerated
        && is_key(&document.class.id, b"Dcmn")
        && document
            .key
            .as_ref()
            .is_some_and(|key| is_key(key, b"Ordn"))
        && matches!(&document.value, ReferenceValue::Key(key) if is_key(key, b"Trgt"));
    if !targets_scale {
        return None;
    }
    descriptor.items.iter().position(|(key, value)| {
        is_key(key, b"T   ")
            && matches!(value, ActionValue::Descriptor(body) if is_key(&body.class.id, SCALE_CLASS))
    })
}

fn decode_resource(data: &[u8]) -> Option<(ActionDescriptor, usize, MeasurementScale)> {
    let descriptor = read_action_descriptor(data.strip_prefix(&VERSION)?, false).ok()?;
    let index = scale_index(&descriptor)?;
    let ActionValue::Descriptor(scale) = &descriptor.items[index].1 else {
        return None;
    };
    let [Some(ActionValue::Integer(pixels)), Some(ActionValue::Double(logical)), Some(ActionValue::Text(units))] =
        SCALE_KEYS.map(|key| {
            scale
                .items
                .iter()
                .find(|(id, _)| is_key(id, key))
                .map(|(_, value)| value)
        })
    else {
        return None;
    };
    let scale =
        MeasurementScale::new(*pixels, *logical, units.strip_suffix('\0').unwrap_or(units))?;
    Some((descriptor, index, scale))
}

pub(crate) fn resolve(doc: &mut Document) {
    let (resources, _) = decode_section(&doc.image_resources);
    if let Some(resource) = scale_resource(&resources) {
        doc.measurement_scale = decode_resource(&resource.data)
            .map_or(DocumentMeasurementScale::Opaque, |(_, _, scale)| {
                DocumentMeasurementScale::Imported(scale)
            });
    }
}

fn scale_values(scale: &MeasurementScale) -> [ActionValue; 3] {
    [
        ActionValue::Integer(scale.pixel_length()),
        ActionValue::Double(scale.logical_length()),
        ActionValue::Text(format!("{}\0", scale.units())),
    ]
}

fn descriptor(class_id: ActionKey, items: Vec<(ActionKey, ActionValue)>) -> ActionDescriptor {
    ActionDescriptor {
        name: "\0".into(),
        class: scale_class(class_id),
        items,
    }
}

fn scale_class(id: ActionKey) -> ActionClass {
    ActionClass {
        name: "\0".into(),
        id,
    }
}

fn new_descriptor(scale: &MeasurementScale) -> ActionDescriptor {
    let measurement_scale = ActionKey::String(SCALE_CLASS.to_vec());
    let target = ActionValue::Reference(vec![
        ActionReferenceItem {
            form: ReferenceForm::Property,
            class: scale_class(ActionKey::Code(*b"Prpr")),
            key: None,
            value: ReferenceValue::Key(measurement_scale.clone()),
        },
        ActionReferenceItem {
            form: ReferenceForm::Enumerated,
            class: scale_class(ActionKey::Code(*b"Dcmn")),
            key: Some(ActionKey::Code(*b"Ordn")),
            value: ReferenceValue::Key(ActionKey::Code(*b"Trgt")),
        },
    ]);
    let items = SCALE_KEYS
        .into_iter()
        .map(|key| ActionKey::String(key.to_vec()))
        .zip(scale_values(scale))
        .collect();
    let body = descriptor(measurement_scale, items);
    descriptor(
        ActionKey::Code(*b"null"),
        vec![
            (ActionKey::Code(*b"null"), target),
            (
                ActionKey::Code(*b"T   "),
                ActionValue::Descriptor(Box::new(body)),
            ),
        ],
    )
}

fn encode_descriptor(descriptor: &ActionDescriptor) -> Vec<u8> {
    let mut data = VERSION.to_vec();
    write_action_descriptor_to(descriptor, false, &mut data);
    data
}

fn replacement_resource(
    scale: &MeasurementScale,
    original: Option<&ImageResource>,
) -> ImageResource {
    // Retain fields in readable descriptors that the calibration editor does not expose.
    if let Some((original, (mut descriptor, index, _))) = original
        .and_then(|resource| decode_resource(&resource.data).map(|decoded| (resource, decoded)))
    {
        if let ActionValue::Descriptor(body) = &mut descriptor.items[index].1 {
            for (key, new_value) in SCALE_KEYS.into_iter().zip(scale_values(scale)) {
                if let Some((_, value)) = body.items.iter_mut().find(|(id, _)| is_key(id, key)) {
                    *value = new_value;
                }
            }
        }
        return frame_image_resource(
            MEASUREMENT_SCALE_RESOURCE,
            &original.name,
            &encode_descriptor(&descriptor),
        );
    }
    frame_image_resource(
        MEASUREMENT_SCALE_RESOURCE,
        "",
        &encode_descriptor(&new_descriptor(scale)),
    )
}

pub(crate) fn resources_for_output(doc: &Document, section: Vec<u8>) -> Vec<u8> {
    let scale = match &doc.measurement_scale {
        DocumentMeasurementScale::Applied(scale) => Some(scale),
        DocumentMeasurementScale::Reset | DocumentMeasurementScale::Resampled => None,
        _ => return section,
    };
    let (mut resources, consumed) = decode_section(&section);
    let mut replacement =
        scale.map(|scale| replacement_resource(scale, scale_resource(&resources)));
    resources.retain_mut(|resource| {
        if resource.id != MEASUREMENT_SCALE_RESOURCE {
            return true;
        }
        if let Some(updated) = replacement.take() {
            *resource = updated;
            return true;
        }
        false
    });
    resources.extend(replacement);
    let mut updated = encode_image_resources(&resources);
    updated.extend_from_slice(&section[consumed..]);
    updated
}
