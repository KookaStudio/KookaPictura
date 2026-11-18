use super::*;
use crate::{
    decode_image_resources, frame_image_resource, read_psd, read_psd_lazy, write_psb, write_psd,
    ImageBudget, OpenOptions,
};
use pictura_core::{BitDepth, ColorMode, Guide, GuideOrientation, MeasurementScale};

fn document_with_opaque_scale(payload: &[u8]) -> Document {
    let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    let mut section = encode_image_resources(&[
        frame_image_resource(1998, "before", b"a"),
        frame_image_resource(MEASUREMENT_SCALE_RESOURCE, "unknown scale", payload),
        frame_image_resource(1999, "after", b"bc"),
    ]);
    section.extend_from_slice(b"tail");
    doc.image_resources = section.into();
    doc
}

#[test]
fn measurement_scale_roundtrips_eager_and_lazy_psd_psb() {
    for write in [write_psd, write_psb] {
        let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
        let fresh = read_psd(&write(&doc).unwrap()).unwrap();
        assert!(scale_resource(&decode_image_resources(&fresh)).is_none());

        let scale = MeasurementScale::new(75, 2.5, "µm 🔬").unwrap();
        assert!(doc.measurement_scale.apply(scale.clone()));
        let bytes = write(&doc).unwrap();
        let mut eager = read_psd(&bytes).unwrap();
        let (lazy, _) =
            read_psd_lazy(&bytes, &OpenOptions::default(), ImageBudget::default()).unwrap();
        assert_eq!(eager.measurement_scale.custom(), Some(&scale));
        assert_eq!(lazy.measurement_scale, eager.measurement_scale);
        let resources = decode_image_resources(&eager);
        let payload = &scale_resource(&resources).unwrap().data;
        assert_eq!(&payload[..4], &16u32.to_be_bytes());
        let descriptor = read_action_descriptor(&payload[4..], false).unwrap();
        assert_eq!(descriptor.class.id, ActionKey::Code(*b"null"));
        let ActionValue::Reference(target) = &descriptor.items[0].1 else {
            panic!("expected the document property reference");
        };
        assert_eq!(target.len(), 2);
        assert_eq!(target[0].form, ReferenceForm::Property);
        assert_eq!(target[0].class.id, ActionKey::Code(*b"Prpr"));
        assert_eq!(
            target[0].value,
            ReferenceValue::Key(ActionKey::String(b"measurementScale".to_vec()))
        );
        assert_eq!(target[1].form, ReferenceForm::Enumerated);
        assert_eq!(target[1].class.id, ActionKey::Code(*b"Dcmn"));
        assert_eq!(target[1].key, Some(ActionKey::Code(*b"Ordn")));
        assert_eq!(
            target[1].value,
            ReferenceValue::Key(ActionKey::Code(*b"Trgt"))
        );
        let ActionValue::Descriptor(body) = &descriptor.items[1].1 else {
            panic!("expected a nested scale descriptor");
        };
        assert_eq!(
            body.class.id,
            ActionKey::String(b"measurementScale".to_vec())
        );
        assert_eq!(
            body.items,
            [
                (
                    ActionKey::String(b"pixelLength".to_vec()),
                    ActionValue::Integer(75)
                ),
                (
                    ActionKey::String(b"logicalLength".to_vec()),
                    ActionValue::Double(2.5)
                ),
                (
                    ActionKey::String(b"logicalUnits".to_vec()),
                    ActionValue::Text("µm 🔬\0".into())
                ),
            ]
        );
        assert!(eager.measurement_scale.reset());
        let reset = read_psd(&write(&eager).unwrap()).unwrap();
        assert!(scale_resource(&decode_image_resources(&reset)).is_none());
        assert_eq!(reset.measurement_scale, DocumentMeasurementScale::Absent);
    }
}

#[test]
fn measurement_scale_preserves_untouched_resource_bytes() {
    let valid = MeasurementScale::new(50, 1.0, "mm").unwrap();
    let mut invalid = new_descriptor(&valid);
    let ActionValue::Descriptor(body) = &mut invalid.items[1].1 else {
        panic!("expected scale descriptor");
    };
    body.items[0].1 = ActionValue::Integer(0);
    let invalid = encode_descriptor(&invalid);
    for payload in [&[0, 0, 0, 17][..], &[0, 0, 0, 16, 0][..], &invalid] {
        let input = document_with_opaque_scale(payload);
        let mut opened = read_psd(&write_psd(&input).unwrap()).unwrap();
        assert_eq!(opened.measurement_scale, DocumentMeasurementScale::Opaque);
        opened
            .composite
            .replace_data(vec![127; 16 * 16 * 3].into())
            .unwrap();
        let resaved = read_psd(&write_psd(&opened).unwrap()).unwrap();
        assert_eq!(input.image_resources, resaved.image_resources);
    }
    let scale = MeasurementScale::new(75, 2.5, "µm").unwrap();
    let mut descriptor = new_descriptor(&scale);
    let scale_index = scale_index(&descriptor).unwrap();
    let ActionValue::Descriptor(body) = &mut descriptor.items[scale_index].1 else {
        panic!("expected scale descriptor");
    };
    body.items.push((
        ActionKey::String(b"fixtureExtra".to_vec()),
        ActionValue::Boolean(true),
    ));
    let original = document_with_opaque_scale(&encode_descriptor(&descriptor));
    let mut loaded = read_psd(&write_psd(&original).unwrap()).unwrap();
    assert_eq!(loaded.measurement_scale.custom(), Some(&scale));
    let kept = read_psd(&write_psd(&loaded).unwrap()).unwrap();
    assert_eq!(kept.image_resources, original.image_resources);

    let new_scale = MeasurementScale::new(40, 9.5, "mm").unwrap();
    assert!(loaded.measurement_scale.apply(new_scale.clone()));
    let edited = read_psd(&write_psd(&loaded).unwrap()).unwrap();
    assert_eq!(edited.measurement_scale.custom(), Some(&new_scale));
    let resources = decode_image_resources(&edited);
    let (descriptor, index, _) =
        decode_resource(&scale_resource(&resources).unwrap().data).unwrap();
    let ActionValue::Descriptor(body) = &descriptor.items[index].1 else {
        panic!("expected scale descriptor");
    };
    assert_eq!(body.items.last().unwrap().1, ActionValue::Boolean(true));
}

#[test]
fn measurement_scale_replaces_or_resets_opaque_resource() {
    let input = document_with_opaque_scale(b"unsupported");
    let mut opened = read_psd(&write_psd(&input).unwrap()).unwrap();
    let scale = MeasurementScale::new(50, 1.0, "µm").unwrap();
    let mut replaced = opened.clone();
    assert!(replaced.measurement_scale.apply(scale.clone()));
    let replaced = read_psd(&write_psd(&replaced).unwrap()).unwrap();
    assert_eq!(replaced.measurement_scale.custom(), Some(&scale));
    let original = decode_image_resources(&input);
    let updated = decode_image_resources(&replaced);
    assert_eq!(updated.len(), original.len());
    assert_eq!(updated[0], original[0]);
    assert_eq!(updated[2], original[2]);
    assert!(replaced.image_resources.ends_with(b"tail"));

    assert!(opened.measurement_scale.reset());
    let resaved = read_psd(&write_psd(&opened).unwrap()).unwrap();
    assert_eq!(resaved.measurement_scale, DocumentMeasurementScale::Absent);
    let remaining: Vec<_> = decode_image_resources(&input)
        .into_iter()
        .filter(|resource| resource.id != MEASUREMENT_SCALE_RESOURCE)
        .collect();
    let mut expected = encode_image_resources(&remaining);
    expected.extend_from_slice(b"tail");
    assert_eq!(&*resaved.image_resources, expected);
}

#[test]
fn measurement_scale_preserves_bytes_with_guide_edit() {
    let input = document_with_opaque_scale(b"unsupported");
    let original = decode_image_resources(&input).remove(1);
    let mut opened = read_psd(&write_psd(&input).unwrap()).unwrap();
    let guide = Guide {
        orientation: GuideOrientation::Vertical,
        position: 5.5,
    };
    opened.guides.push(guide);
    let resaved = read_psd(&write_psd(&opened).unwrap()).unwrap();
    assert_eq!(resaved.guides, [guide]);
    let scale = decode_image_resources(&resaved)
        .into_iter()
        .find(|resource| resource.id == MEASUREMENT_SCALE_RESOURCE)
        .unwrap();
    assert_eq!(scale.raw, original.raw);
    assert!(resaved.image_resources.ends_with(b"tail"));
}
