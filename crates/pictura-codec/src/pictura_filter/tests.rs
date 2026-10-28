use super::*;

fn steps() -> Vec<FilterStep> {
    vec![
        FilterStep {
            kind: "gaussian-blur".into(),
            slots: vec![2.5],
        },
        FilterStep {
            kind: "add-noise".into(),
            slots: vec![12.0, 1.0, 0.0],
        },
    ]
}

#[test]
fn steps_round_trip() {
    let options = encode_pictura_filter(&steps(), false);
    assert_eq!(decode_pictura_filter(&options), Some((steps(), false)));
}

#[test]
fn gallery_flag_round_trips() {
    let options = encode_pictura_filter(&steps(), true);
    let (decoded, gallery) = decode_pictura_filter(&options).expect("decodes");
    assert!(gallery);
    assert_eq!(decoded, steps());
}

#[test]
fn malformed_options_decode_none() {
    assert!(decode_pictura_filter(&[]).is_none());
    assert!(decode_pictura_filter(&encode_pictura_filter(&[], false)).is_none());
    let wrong = descriptor::write_descriptor(&DescValue::Object {
        name: String::new(),
        class_id: b"Pictura Filter".to_vec(),
        items: vec![
            (b"Vrsn".to_vec(), DescValue::Long(VERSION)),
            (b"Glry".to_vec(), DescValue::Long(1)),
            (b"Stck".to_vec(), DescValue::List(Vec::new())),
        ],
    });
    assert!(decode_pictura_filter(&wrong).is_none());
    let bad_slot = descriptor::write_descriptor(&DescValue::Object {
        name: String::new(),
        class_id: b"Pictura Filter".to_vec(),
        items: vec![
            (b"Vrsn".to_vec(), DescValue::Long(VERSION)),
            (b"Glry".to_vec(), DescValue::Bool(false)),
            (
                b"Stck".to_vec(),
                DescValue::List(vec![DescValue::Object {
                    name: String::new(),
                    class_id: b"x".to_vec(),
                    items: vec![
                        (b"Kind".to_vec(), DescValue::Text("blur\0".into())),
                        (b"Prms".to_vec(), DescValue::List(vec![DescValue::Long(1)])),
                    ],
                }]),
            ),
        ],
    });
    assert!(decode_pictura_filter(&bad_slot).is_none());
}
