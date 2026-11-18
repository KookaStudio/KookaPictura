use super::*;
use crate::{read_psd, write_psb, write_psd};
use pictura_core::{BitDepth, ColorMode};

fn document(resources: &[ImageResource]) -> Document {
    let mut doc = Document::new(4, 3, ColorMode::Rgb, BitDepth::Eight);
    doc.image_resources = encode_image_resources(resources).into();
    doc
}

#[test]
fn print_resources_round_trip_psd_and_psb() {
    let mut info = descriptor(b"printOutput");
    info.items
        .push((ActionKey::Code(*b"PstS"), ActionValue::Boolean(false)));
    info.items
        .push((ActionKey::Code(*b"Nm  "), text_value("Test printer".into())));
    let extension = (key(b"fixtureSetting"), ActionValue::RawData(vec![5, 4, 3]));
    info.items.push(extension.clone());
    let os_data = frame_image_resource(1085, "driver", &[1, 3, 5, 7]);
    let other = frame_image_resource(4000, "unrelated", &[2, 4, 6]);
    let flags = frame_image_resource(1011, "", &[0, 0, 0, 0, 1, 0, 0, 0]);
    let mut doc = document(&[
        pack(PRINT_INFO, "info", &info),
        flags,
        os_data.clone(),
        other.clone(),
    ]);
    let original = read_print_settings(&doc);
    let changed = PrintSettings {
        intent: Some(Intent::Saturation),
        corner_crop: Some(true),
        border: Some(7.5),
        ..original.clone()
    };
    assert!(update_print_settings(&mut doc, &original, &changed).unwrap());
    for write in [
        write_psd as fn(&Document) -> Result<Vec<u8>, PsdError>,
        write_psb,
    ] {
        let read = read_psd(&write(&doc).unwrap()).unwrap();
        assert_eq!(read_print_settings(&read), changed);
        let resources = crate::decode_image_resources(&read);
        let find = |id| resources.iter().find(|resource| resource.id == id).unwrap();
        assert_eq!(find(os_data.id).raw, os_data.raw);
        assert_eq!(find(other.id).raw, other.raw);
        assert_eq!(find(1011).data, [0, 1, 0, 0, 1, 0, 0, 0]);
        let info = unpack(find(1082), b"printOutput").unwrap();
        assert!(info.items.contains(&extension));
        assert_eq!(
            value(&info, b"Inte"),
            Some(&ActionValue::Enumerated {
                kind: ActionKey::Code(*b"Inte"),
                value: ActionKey::Code(*b"Grp "),
            })
        );
        let marks = unpack(find(1083), b"printOutputOptions").unwrap();
        assert_eq!(value(&marks, b"CrnC"), Some(&ActionValue::Boolean(true)));
    }
}

#[test]
fn opaque_print_resources_survive_save() {
    let bad = frame_image_resource(1082, "opaque", &[0, 0, 0, 16, 9]);
    let mut doc = document(std::slice::from_ref(&bad));
    let baseline = read_print_settings(&doc);
    let updated = PrintSettings {
        manage_colors: Some(false),
        ..baseline.clone()
    };
    assert!(update_print_settings(&mut doc, &baseline, &updated).is_err());
    assert!(!update_print_settings(&mut doc, &baseline, &baseline).unwrap());
    let reloaded = read_psd(&write_psd(&doc).unwrap()).unwrap();
    let actual = crate::decode_image_resources(&reloaded);
    assert_eq!(actual.iter().find(|r| r.id == bad.id).unwrap().raw, bad.raw);
}
