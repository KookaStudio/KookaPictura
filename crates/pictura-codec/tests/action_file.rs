use std::path::{Path, PathBuf};

use pictura_codec::action_descriptor::{
    read_action_descriptor, write_action_descriptor, ActionClass, ActionDescriptor, ActionKey,
    ActionReferenceItem, ActionValue, ObjectArray, ObjectArrayItem, ReferenceForm, ReferenceValue,
};
use pictura_codec::action_file::{
    read_atn, read_psp, write_atn, write_psp, RawAction, RawActionSet, RawAtn, RawEventId, RawStep,
};

fn key(code: &[u8; 4]) -> ActionKey {
    ActionKey::Code(*code)
}

fn text_key(value: &[u8]) -> ActionKey {
    ActionKey::String(value.to_vec())
}

fn class(name: &str, code: &[u8; 4]) -> ActionClass {
    ActionClass {
        name: name.to_string(),
        id: key(code),
    }
}

fn descriptor(
    name: &str,
    code: &[u8; 4],
    items: Vec<(ActionKey, ActionValue)>,
) -> ActionDescriptor {
    ActionDescriptor {
        name: name.to_string(),
        class: class(name, code),
        items,
    }
}

fn units(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_f64(out: &mut Vec<u8>, value: f64) {
    out.extend_from_slice(&value.to_bits().to_be_bytes());
}

fn push_units(out: &mut Vec<u8>, units: &[u16]) {
    push_u32(out, units.len() as u32);
    for unit in units {
        push_u16(out, *unit);
    }
}

fn push_utf16(out: &mut Vec<u8>, value: &str) {
    let units: Vec<u16> = value.encode_utf16().collect();
    push_units(out, &units);
}

fn push_code_key(out: &mut Vec<u8>, code: &[u8; 4]) {
    push_u32(out, 0);
    out.extend_from_slice(code);
}

fn push_string_key(out: &mut Vec<u8>, bytes: &[u8]) {
    push_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

fn push_bytes_field(out: &mut Vec<u8>, bytes: &[u8]) {
    push_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

fn push_invalid_utf16(out: &mut Vec<u8>) {
    push_units(out, &[0xD800]);
}

fn descriptor_with_item(key_code: &[u8; 4], value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_utf16(&mut bytes, "Root\0");
    push_code_key(&mut bytes, b"root");
    push_u32(&mut bytes, 1);
    push_code_key(&mut bytes, key_code);
    bytes.extend_from_slice(value);
    bytes
}

fn raw_step(
    expanded: bool,
    enabled: bool,
    with_dialog: bool,
    dialog_options: u8,
    event_id: RawEventId,
    display_name: &[u8],
    descriptor: Option<ActionDescriptor>,
) -> RawStep {
    RawStep {
        expanded,
        enabled,
        with_dialog,
        dialog_options,
        event_id,
        display_name: display_name.to_vec(),
        descriptor,
    }
}

fn fixture_set() -> RawActionSet {
    let select = raw_step(
        true,
        true,
        true,
        1,
        RawEventId::Long(*b"slct"),
        b"Select",
        Some(descriptor(
            "slct\0",
            b"slct",
            vec![(
                key(b"null"),
                ActionValue::Reference(vec![ActionReferenceItem {
                    form: ReferenceForm::Property,
                    class: class("\0", b"Lyr "),
                    key: None,
                    value: ReferenceValue::Key(text_key(b"name")),
                }]),
            )],
        )),
    );
    let test = raw_step(
        false,
        true,
        false,
        2,
        RawEventId::Text(b"pictura.test".to_vec()),
        b"Test\0display",
        Some(descriptor(
            "pictura.test\0",
            b"pict",
            vec![(key(b"Nmb "), ActionValue::Integer(7))],
        )),
    );
    let empty = raw_step(
        true,
        false,
        false,
        3,
        RawEventId::Text(b"opaque.event".to_vec()),
        b"Opaque",
        None,
    );
    let save = raw_step(
        false,
        true,
        false,
        0,
        RawEventId::Long(*b"save"),
        b"Save",
        None,
    );
    RawActionSet {
        name: units("Set A"),
        expanded: true,
        actions: vec![
            RawAction {
                function_key: 3,
                shift: true,
                ctrl: false,
                color: 5,
                name: units("Action One"),
                expanded: true,
                steps: vec![select, test, empty],
            },
            RawAction {
                function_key: 0,
                shift: false,
                ctrl: true,
                color: 0,
                name: units("Action Two"),
                expanded: false,
                steps: vec![save],
            },
        ],
    }
}

fn fixture_atn() -> RawAtn {
    RawAtn {
        set: fixture_set(),
        trailing: Vec::new(),
    }
}

fn hand_built_atn() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u32(&mut bytes, 16);
    push_units(&mut bytes, &units("Set A"));
    bytes.push(1);
    push_u32(&mut bytes, 1);
    push_u16(&mut bytes, 3);
    bytes.push(1);
    bytes.push(1);
    push_u16(&mut bytes, 5);
    push_units(&mut bytes, &units("Action"));
    bytes.push(1);
    push_u32(&mut bytes, 1);
    bytes.push(1);
    bytes.push(0);
    bytes.push(1);
    bytes.push(2);
    bytes.extend_from_slice(b"long");
    bytes.extend_from_slice(b"test");
    push_bytes_field(&mut bytes, b"Test");
    push_i32(&mut bytes, -1);
    push_utf16(&mut bytes, "Root\0");
    push_code_key(&mut bytes, b"root");
    push_u32(&mut bytes, 4);
    push_string_key(&mut bytes, b"StrK");
    bytes.extend_from_slice(b"TEXT");
    push_utf16(&mut bytes, "hi\0");
    push_code_key(&mut bytes, b"Code");
    bytes.extend_from_slice(b"Pth ");
    push_bytes_field(&mut bytes, b"pathdata");
    push_code_key(&mut bytes, b"obar");
    bytes.extend_from_slice(b"ObAr");
    push_u32(&mut bytes, 2);
    push_utf16(&mut bytes, "Pnt \0");
    push_code_key(&mut bytes, b"Pnt ");
    push_u32(&mut bytes, 2);
    push_code_key(&mut bytes, b"Hrzn");
    bytes.extend_from_slice(b"UnFl");
    bytes.extend_from_slice(b"#Pxl");
    push_u32(&mut bytes, 2);
    push_f64(&mut bytes, 1.5);
    push_f64(&mut bytes, 2.5);
    push_code_key(&mut bytes, b"Vrtc");
    bytes.extend_from_slice(b"UnFl");
    bytes.extend_from_slice(b"#Prc");
    push_u32(&mut bytes, 2);
    push_f64(&mut bytes, -1.0);
    push_f64(&mut bytes, 3.0);
    push_code_key(&mut bytes, b"bool");
    bytes.extend_from_slice(b"bool");
    bytes.push(1);
    bytes
}

#[test]
fn af_atn_hand_built_fixture() {
    let bytes = hand_built_atn();
    let decoded = read_atn(&bytes).expect("hand-built fixture reads");

    assert_eq!(decoded.set.name, units("Set A"));
    assert!(decoded.set.expanded);
    assert_eq!(decoded.set.actions.len(), 1);

    let action = &decoded.set.actions[0];
    assert_eq!(action.function_key, 3);
    assert!(action.shift);
    assert!(action.ctrl);
    assert_eq!(action.color, 5);
    assert_eq!(action.name, units("Action"));
    assert!(action.expanded);
    assert_eq!(action.steps.len(), 1);

    let step = &action.steps[0];
    assert!(step.expanded);
    assert!(!step.enabled);
    assert!(step.with_dialog);
    assert_eq!(step.dialog_options, 2);
    assert_eq!(step.event_id, RawEventId::Long(*b"test"));
    assert_eq!(step.display_name, b"Test".to_vec());

    let descriptor = step.descriptor.as_ref().expect("descriptor present");
    assert_eq!(descriptor.name, "Root\0");
    assert_eq!(descriptor.class, class("Root\0", b"root"));
    assert_eq!(descriptor.items.len(), 4);
    assert_eq!(descriptor.items[0].0, text_key(b"StrK"));
    assert_eq!(descriptor.items[0].1, ActionValue::Text("hi\0".to_string()));
    assert_eq!(descriptor.items[1].0, key(b"Code"));
    assert_eq!(
        descriptor.items[1].1,
        ActionValue::Path(b"pathdata".to_vec())
    );

    let ActionValue::ObjectArray(array) = &descriptor.items[2].1 else {
        panic!("object array expected");
    };
    assert_eq!(array.class(), &class("Pnt \0", b"Pnt "));
    assert_eq!(array.object_count(), 2);
    assert_eq!(array.items().len(), 2);
    assert_eq!(array.items()[0].key, key(b"Hrzn"));
    assert_eq!(array.items()[0].unit, *b"#Pxl");
    assert_eq!(array.items()[0].values, vec![1.5, 2.5]);
    assert_eq!(array.items()[1].key, key(b"Vrtc"));
    assert_eq!(array.items()[1].unit, *b"#Prc");
    assert_eq!(array.items()[1].values, vec![-1.0, 3.0]);

    assert_eq!(descriptor.items[3].0, key(b"bool"));
    assert_eq!(descriptor.items[3].1, ActionValue::Boolean(true));

    assert_eq!(write_atn(&decoded), bytes);
}

#[test]
fn af_atn_read() {
    let source = fixture_atn();
    let decoded = read_atn(&write_atn(&source)).expect("valid fixture reads");

    assert_eq!(decoded.set.name, units("Set A"));
    assert!(decoded.set.expanded);
    assert_eq!(decoded.set.actions.len(), 2);

    let first = &decoded.set.actions[0];
    assert_eq!(first.function_key, 3);
    assert!(first.shift);
    assert!(!first.ctrl);
    assert_eq!(first.color, 5);
    assert_eq!(first.name, units("Action One"));
    assert!(first.expanded);
    assert_eq!(first.steps.len(), 3);

    let select = &first.steps[0];
    assert!(select.expanded);
    assert!(select.enabled);
    assert!(select.with_dialog);
    assert_eq!(select.dialog_options, 1);
    assert_eq!(select.event_id, RawEventId::Long(*b"slct"));
    assert_eq!(select.display_name, b"Select".to_vec());
    let select_descriptor = select.descriptor.as_ref().expect("descriptor present");
    assert_eq!(select_descriptor.name, "slct\0");
    assert_eq!(select_descriptor.class.id, key(b"slct"));

    let test = &first.steps[1];
    assert!(!test.expanded);
    assert_eq!(test.event_id, RawEventId::Text(b"pictura.test".to_vec()));
    assert_eq!(test.display_name, b"Test\0display".to_vec());
    assert_eq!(
        test.descriptor.as_ref().expect("descriptor present").items,
        vec![(key(b"Nmb "), ActionValue::Integer(7))]
    );

    let empty = &first.steps[2];
    assert!(!empty.enabled);
    assert!(empty.descriptor.is_none());
    assert_eq!(empty.display_name, b"Opaque".to_vec());

    let second = &decoded.set.actions[1];
    assert_eq!(second.function_key, 0);
    assert!(second.ctrl);
    assert_eq!(second.name, units("Action Two"));
    assert_eq!(second.steps[0].event_id, RawEventId::Long(*b"save"));
}

#[test]
fn af_atn_no_magic() {
    let bytes = write_atn(&fixture_atn());
    assert_eq!(
        u32::from_be_bytes(bytes[..4].try_into().expect("version bytes")),
        16
    );
    assert!(!bytes.windows(4).any(|window| window == &b"8BAC"[..]));
    assert!(read_atn(&bytes).is_ok());
}

#[test]
fn af_atn_version() {
    for version in [12u32, 15, 17] {
        let mut bytes = write_atn(&fixture_atn());
        bytes[..4].copy_from_slice(&version.to_be_bytes());
        let failure = read_atn(&bytes).expect_err("wrong version is refused");
        assert!(
            failure.message.contains(version.to_string().as_str()),
            "{}",
            failure.message
        );
        assert!(failure.partial_sets.is_empty());
    }
}

#[test]
fn af_atn_trailing() {
    let trailing = vec![0xde, 0xad, 0xbe, 0xef];
    let mut source = fixture_atn();
    source.trailing = trailing.clone();
    let bytes = write_atn(&source);

    let decoded = read_atn(&bytes).expect("trailing bytes are accepted");
    assert_eq!(decoded.trailing, trailing);
    assert!(write_atn(&decoded).ends_with(&trailing));
    assert_eq!(write_atn(&decoded), bytes);
}

#[test]
fn af_atn_dialog_options() {
    let mut source = fixture_atn();
    source.set.actions.truncate(1);
    source.set.actions[0].steps = (0..=4u8)
        .map(|options| {
            raw_step(
                true,
                true,
                options % 2 == 0,
                options,
                RawEventId::Long(*b"test"),
                b"Options",
                None,
            )
        })
        .collect();

    let decoded = read_atn(&write_atn(&source)).expect("options 0-4 read");
    let options: Vec<u8> = decoded.set.actions[0]
        .steps
        .iter()
        .map(|step| step.dialog_options)
        .collect();
    assert_eq!(options, vec![0, 1, 2, 3, 0]);

    let rewritten = read_atn(&write_atn(&decoded)).expect("rewritten options read");
    let options: Vec<u8> = rewritten.set.actions[0]
        .steps
        .iter()
        .map(|step| step.dialog_options)
        .collect();
    assert_eq!(options, vec![0, 1, 2, 3, 0]);
}

#[test]
fn af_atn_roundtrip() {
    let source = fixture_atn();
    let bytes = write_atn(&source);
    let decoded = read_atn(&bytes).expect("round-trip read");
    assert_eq!(decoded, source);
    assert_eq!(write_atn(&decoded), bytes);
}

#[test]
fn af_atn_id_form() {
    let source = descriptor(
        "\0",
        b"null",
        vec![
            (text_key(b"ABCD"), ActionValue::Integer(1)),
            (key(b"ABCD"), ActionValue::Integer(2)),
        ],
    );
    let bytes = write_action_descriptor(&source, true);

    let string_form = [0u8, 0, 0, 4, b'A', b'B', b'C', b'D'];
    let code_form = [0u8, 0, 0, 0, b'A', b'B', b'C', b'D'];
    assert!(bytes.windows(8).any(|window| window == &string_form[..]));
    assert!(bytes.windows(8).any(|window| window == &code_form[..]));

    let decoded = read_action_descriptor(&bytes, true).expect("identifier forms read");
    assert!(matches!(decoded.items[0].0, ActionKey::String(_)));
    assert!(matches!(decoded.items[1].0, ActionKey::Code(_)));
    assert_eq!(decoded, source);
    assert_eq!(write_action_descriptor(&decoded, true), bytes);
}

#[test]
fn af_atn_order() {
    let source = descriptor(
        "\0",
        b"null",
        vec![
            (key(b"zzzz"), ActionValue::Integer(1)),
            (key(b"aaaa"), ActionValue::Integer(2)),
            (key(b"zzzz"), ActionValue::Integer(3)),
        ],
    );
    let bytes = write_action_descriptor(&source, true);
    let decoded = read_action_descriptor(&bytes, true).expect("ordered items read");

    let keys: Vec<ActionKey> = decoded.items.iter().map(|(key, _)| key.clone()).collect();
    assert_eq!(keys, vec![key(b"zzzz"), key(b"aaaa"), key(b"zzzz")]);
    assert_eq!(decoded, source);
    assert_eq!(write_action_descriptor(&decoded, true), bytes);
}

#[test]
fn af_atn_unknown_save() {
    let mut source = fixture_atn();
    source.set.actions.truncate(1);
    source.set.actions[0].steps = vec![
        raw_step(
            true,
            true,
            false,
            0,
            RawEventId::Text(b"pictura.unknown".to_vec()),
            b"Unknown\0display",
            Some(descriptor(
                "unkn\0",
                b"unkn",
                vec![(key(b"Val "), ActionValue::Integer(9))],
            )),
        ),
        raw_step(
            false,
            true,
            true,
            2,
            RawEventId::Long(*b"zzzz"),
            b"Long unknown",
            None,
        ),
    ];

    let bytes = write_atn(&source);
    let decoded = read_atn(&bytes).expect("unknown events load");
    assert_eq!(decoded, source);
    assert_eq!(write_atn(&decoded), bytes);
}

#[test]
fn af_psp_roundtrip() {
    let first = fixture_set();
    let mut second = fixture_set();
    second.name = units("Second Set");
    second.actions.reverse();

    let bytes = write_psp(&[first.clone(), second.clone()]);
    let sets = read_psp(&bytes).expect("two-set palette reads");
    assert_eq!(sets, vec![first, second]);
}

#[test]
fn af_psp_count() {
    let set = fixture_set();
    let mut bytes = write_psp(std::slice::from_ref(&set));
    bytes[4..8].copy_from_slice(&2u32.to_be_bytes());

    let failure = read_psp(&bytes).expect_err("declared extra set is truncated");
    assert_eq!(failure.partial_sets, vec![set]);
    assert!(failure.message.contains("palette"));
}

#[test]
fn af_psp_partial() {
    let first = fixture_set();
    let mut second = fixture_set();
    second.name = units("Second Set");
    let mut bytes = write_psp(&[first.clone(), second]);
    let first_only = write_psp(std::slice::from_ref(&first));
    assert!(bytes.len() > first_only.len());
    bytes.truncate(bytes.len() - 5);

    let failure = read_psp(&bytes).expect_err("truncated second set is reported");
    assert_eq!(failure.partial_sets, vec![first]);
    assert!(failure.offset >= first_only.len());
    assert!(failure.message.contains("palette"));
}

#[test]
fn af_psp_version() {
    let mut bytes = write_psp(&[fixture_set()]);
    bytes[..4].copy_from_slice(&15u32.to_be_bytes());
    let failure = read_psp(&bytes).expect_err("wrong palette version is refused");
    assert!(failure.message.contains("15"));
    assert!(failure.partial_sets.is_empty());
}

fn path_payload(text: &str) -> Vec<u8> {
    let units: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let byte_len = (units.len() * 2) as u32;
    let mut payload = Vec::new();
    payload.extend_from_slice(b"utxt");
    payload.extend_from_slice(&byte_len.to_le_bytes());
    payload.extend_from_slice(&(units.len() as u32).to_le_bytes());
    for unit in units {
        payload.extend_from_slice(&unit.to_le_bytes());
    }
    payload
}

#[test]
fn af_desc_values() {
    let nested = descriptor(
        "nested\0",
        b"nest",
        vec![(key(b"Nmb "), ActionValue::Integer(3))],
    );
    let object_array = ObjectArray::new(
        class("Pnt \0", b"Pnt "),
        2,
        vec![
            ObjectArrayItem {
                key: key(b"Hrzn"),
                unit: *b"#Pxl",
                values: vec![1.0, 3.0],
            },
            ObjectArrayItem {
                key: key(b"Vrtc"),
                unit: *b"#Pxl",
                values: vec![2.0, 4.0],
            },
        ],
    )
    .expect("uniform object array");
    let source = descriptor(
        "Root\0",
        b"root",
        vec![
            (
                key(b"ref "),
                ActionValue::Reference(vec![ActionReferenceItem {
                    form: ReferenceForm::Class,
                    class: class("\0", b"Lyr "),
                    key: None,
                    value: ReferenceValue::None,
                }]),
            ),
            (
                key(b"objc"),
                ActionValue::Descriptor(Box::new(nested.clone())),
            ),
            (
                key(b"glbo"),
                ActionValue::GlobalObject(Box::new(nested.clone())),
            ),
            (
                key(b"list"),
                ActionValue::List(vec![
                    ActionValue::Integer(1),
                    ActionValue::Text("two\0".to_string()),
                ]),
            ),
            (key(b"doub"), ActionValue::Double(-1.25)),
            (
                key(b"untf"),
                ActionValue::UnitFloat {
                    unit: *b"#Prc",
                    value: 50.0,
                },
            ),
            (
                key(b"unfl"),
                ActionValue::UnitFloats {
                    unit: *b"#Rlt",
                    values: vec![10.0, 20.0],
                },
            ),
            (key(b"text"), ActionValue::Text("hello\0".to_string())),
            (
                key(b"enum"),
                ActionValue::Enumerated {
                    kind: key(b"Ornt"),
                    value: key(b"Hrzn"),
                },
            ),
            (key(b"long"), ActionValue::Integer(-7)),
            (key(b"comp"), ActionValue::LargeInteger(9_000_000_000)),
            (key(b"bool"), ActionValue::Boolean(true)),
            (key(b"clss"), ActionValue::Class(class("\0", b"Lyr "))),
            (key(b"glbc"), ActionValue::GlobalClass(class("\0", b"GlbC"))),
            (key(b"alis"), ActionValue::Alias(vec![1, 2, 3])),
            (key(b"tdta"), ActionValue::RawData(vec![4, 5, 6])),
            (
                key(b"path"),
                ActionValue::Path(path_payload("C:\\tmp\\a.psd")),
            ),
            (key(b"obar"), ActionValue::ObjectArray(object_array)),
        ],
    );

    let bytes = write_action_descriptor(&source, true);
    let decoded = read_action_descriptor(&bytes, true).expect("every value type reads");
    assert_eq!(decoded, source);
    assert_eq!(write_action_descriptor(&decoded, true), bytes);
}

#[test]
fn af_desc_refs() {
    let reference = vec![
        ActionReferenceItem {
            form: ReferenceForm::Property,
            class: class("\0", b"Lyr "),
            key: None,
            value: ReferenceValue::Key(text_key(b"name")),
        },
        ActionReferenceItem {
            form: ReferenceForm::Class,
            class: class("\0", b"Doc "),
            key: None,
            value: ReferenceValue::None,
        },
        ActionReferenceItem {
            form: ReferenceForm::Enumerated,
            class: class("\0", b"Chnl"),
            key: Some(key(b"Chnl")),
            value: ReferenceValue::Key(key(b"Rd  ")),
        },
        ActionReferenceItem {
            form: ReferenceForm::Offset,
            class: class("\0", b"Lyr "),
            key: None,
            value: ReferenceValue::Integer(-2),
        },
        ActionReferenceItem {
            form: ReferenceForm::Identifier,
            class: class("\0", b"Lyr "),
            key: None,
            value: ReferenceValue::Unsigned(u32::MAX),
        },
        ActionReferenceItem {
            form: ReferenceForm::Index,
            class: class("\0", b"Lyr "),
            key: None,
            value: ReferenceValue::Unsigned(7),
        },
        ActionReferenceItem {
            form: ReferenceForm::Name,
            class: class("\0", b"Lyr "),
            key: None,
            value: ReferenceValue::Text("Layer 1\0".to_string()),
        },
    ];
    let source = descriptor(
        "Root\0",
        b"root",
        vec![(key(b"ref "), ActionValue::Reference(reference))],
    );

    let bytes = write_action_descriptor(&source, true);
    let decoded = read_action_descriptor(&bytes, true).expect("every reference form reads");
    assert_eq!(decoded, source);
    assert_eq!(write_action_descriptor(&decoded, true), bytes);
}

#[test]
fn af_desc_idnt_class() {
    let source = descriptor(
        "Root\0",
        b"root",
        vec![(
            key(b"ref "),
            ActionValue::Reference(vec![
                ActionReferenceItem {
                    form: ReferenceForm::Identifier,
                    class: class("\0", b"Lyr "),
                    key: None,
                    value: ReferenceValue::Unsigned(42),
                },
                ActionReferenceItem {
                    form: ReferenceForm::Index,
                    class: class("\0", b"Ptr "),
                    key: None,
                    value: ReferenceValue::Unsigned(7),
                },
            ]),
        )],
    );
    let bytes = write_action_descriptor(&source, true);

    let identifier = [0u8, 0, 0, 0, b'L', b'y', b'r', b' ', 0, 0, 0, 42];
    let index = [0u8, 0, 0, 0, b'P', b't', b'r', b' ', 0, 0, 0, 7];
    assert!(bytes
        .windows(identifier.len())
        .any(|window| window == &identifier[..]));
    assert!(bytes
        .windows(index.len())
        .any(|window| window == &index[..]));

    let atn = RawAtn {
        set: RawActionSet {
            name: units("Set"),
            expanded: true,
            actions: vec![RawAction {
                function_key: 0,
                shift: false,
                ctrl: false,
                color: 0,
                name: units("Action"),
                expanded: true,
                steps: vec![raw_step(
                    true,
                    true,
                    false,
                    0,
                    RawEventId::Long(*b"test"),
                    b"Test",
                    Some(source.clone()),
                )],
            }],
        },
        trailing: Vec::new(),
    };
    let decoded = read_atn(&write_atn(&atn)).expect("class prefixes read from .atn");
    let descriptor = decoded.set.actions[0].steps[0]
        .descriptor
        .as_ref()
        .expect("descriptor present");
    assert_eq!(descriptor, &source);
}

#[test]
fn af_desc_unknown() {
    let source = descriptor(
        "\0",
        b"null",
        vec![(key(b"bad "), ActionValue::Double(1.0))],
    );
    let mut bytes = write_action_descriptor(&source, true);
    let at = bytes
        .windows(4)
        .position(|window| window == &b"doub"[..])
        .expect("double type is present");
    bytes[at..at + 4].copy_from_slice(b"zzzz");

    let failure = read_action_descriptor(&bytes, true).expect_err("unknown type fails closed");
    assert!(failure.message.contains("zzzz"), "{}", failure.message);
    assert_eq!(failure.offset, at);
}

#[test]
fn af_malformed_truncated() {
    let bytes = write_atn(&fixture_atn());
    for cut in 0..bytes.len() {
        assert!(
            read_atn(&bytes[..cut]).is_err(),
            "truncating the action file at {cut} must error"
        );
    }

    let value = descriptor(
        "Root\0",
        b"root",
        vec![(key(b"Text"), ActionValue::Text("hello\0".to_string()))],
    );
    let bytes = write_action_descriptor(&value, true);
    for cut in 0..bytes.len() {
        assert!(
            read_action_descriptor(&bytes[..cut], true).is_err(),
            "truncating the descriptor at {cut} must error"
        );
    }
}

#[test]
fn af_malformed_count() {
    let mut bytes = write_atn(&fixture_atn());
    bytes[4..8].copy_from_slice(&u32::MAX.to_be_bytes());
    let failure = read_atn(&bytes).expect_err("oversized string count is refused");
    assert!(
        failure.message.contains("count") || failure.message.contains("exceeds"),
        "{}",
        failure.message
    );

    let action_count_at = 4 + 4 + units("Set A").len() * 2 + 1;
    let mut bytes = write_atn(&fixture_atn());
    bytes[action_count_at..action_count_at + 4].copy_from_slice(&u32::MAX.to_be_bytes());
    let failure = read_atn(&bytes).expect_err("oversized action count is refused");
    assert!(
        failure.message.contains("count") || failure.message.contains("exceeds"),
        "{}",
        failure.message
    );
}

#[test]
fn af_desc_invalid_utf16_fails_closed() {
    let mut bytes = Vec::new();
    push_invalid_utf16(&mut bytes);
    push_code_key(&mut bytes, b"root");
    push_u32(&mut bytes, 0);
    let failure = read_action_descriptor(&bytes, true).expect_err("descriptor name is checked");
    assert!(failure.message.contains("UTF-16"), "{}", failure.message);

    let mut value = Vec::new();
    value.extend_from_slice(b"type");
    push_invalid_utf16(&mut value);
    let failure = read_action_descriptor(&descriptor_with_item(b"clss", &value), true)
        .expect_err("class name is checked");
    assert!(failure.message.contains("UTF-16"), "{}", failure.message);

    let mut value = Vec::new();
    value.extend_from_slice(b"TEXT");
    push_invalid_utf16(&mut value);
    let failure = read_action_descriptor(&descriptor_with_item(b"text", &value), true)
        .expect_err("TEXT value is checked");
    assert!(failure.message.contains("UTF-16"), "{}", failure.message);

    let mut value = Vec::new();
    value.extend_from_slice(b"obj ");
    push_u32(&mut value, 1);
    value.extend_from_slice(b"name");
    push_utf16(&mut value, "");
    push_code_key(&mut value, b"Lyr ");
    push_invalid_utf16(&mut value);
    let failure = read_action_descriptor(&descriptor_with_item(b"ref ", &value), true)
        .expect_err("reference name is checked");
    assert!(failure.message.contains("UTF-16"), "{}", failure.message);
}

#[test]
fn af_desc_empty_string_key_stays_aligned() {
    let source = descriptor(
        "\0",
        b"null",
        vec![
            (text_key(b""), ActionValue::Integer(1)),
            (key(b"next"), ActionValue::Integer(2)),
        ],
    );
    let bytes = write_action_descriptor(&source, true);
    let decoded =
        read_action_descriptor(&bytes, true).expect("an empty string key is representable");
    assert_eq!(decoded.items[0].0, key(b"\0\0\0\0"));
    assert_eq!(decoded.items[0].1, ActionValue::Integer(1));
    assert_eq!(decoded.items[1].0, key(b"next"));
    assert_eq!(decoded.items[1].1, ActionValue::Integer(2));
    assert_eq!(write_action_descriptor(&decoded, true), bytes);
}

#[test]
fn af_desc_standalone_rejects_trailing_bytes() {
    let source = descriptor("\0", b"null", Vec::new());
    let mut bytes = write_action_descriptor(&source, true);
    let trailing_at = bytes.len();
    bytes.push(0xaa);

    let failure =
        read_action_descriptor(&bytes, true).expect_err("trailing descriptor data is refused");
    assert!(failure.message.contains("trailing"), "{}", failure.message);
    assert_eq!(failure.offset, trailing_at);
}

#[test]
fn af_desc_boolean_keeps_raw_byte() {
    for (raw, expected) in [
        (0u8, ActionValue::Boolean(false)),
        (1u8, ActionValue::Boolean(true)),
        (2u8, ActionValue::BooleanRaw(2)),
    ] {
        let mut value = Vec::new();
        value.extend_from_slice(b"bool");
        value.push(raw);
        let bytes = descriptor_with_item(b"bool", &value);
        let decoded = read_action_descriptor(&bytes, true).expect("boolean reads");
        assert_eq!(decoded.items[0].1, expected);
        assert_eq!(write_action_descriptor(&decoded, true), bytes);
    }
}

#[test]
fn af_desc_object_array_preserves_shared_class_and_uniform_items() {
    let shared = class("Pnt \0", b"Pnt ");
    let array = ObjectArray::new(
        shared.clone(),
        0,
        vec![ObjectArrayItem {
            key: key(b"Hrzn"),
            unit: *b"#Pxl",
            values: Vec::new(),
        }],
    )
    .expect("empty object array is uniform");
    let source = descriptor(
        "Root\0",
        b"root",
        vec![(key(b"obar"), ActionValue::ObjectArray(array))],
    );
    let bytes = write_action_descriptor(&source, true);
    let decoded = read_action_descriptor(&bytes, true).expect("empty object array reads");
    assert_eq!(decoded, source);

    let ActionValue::ObjectArray(array) = &decoded.items[0].1 else {
        panic!("object array expected");
    };
    assert_eq!(array.class(), &shared);
    assert_eq!(array.object_count(), 0);
    assert_eq!(array.items().len(), 1);
    assert!(array.items()[0].values.is_empty());
    assert_eq!(write_action_descriptor(&decoded, true), bytes);

    let non_uniform = ObjectArray::new(
        shared,
        2,
        vec![ObjectArrayItem {
            key: key(b"Hrzn"),
            unit: *b"#Pxl",
            values: vec![1.0],
        }],
    );
    assert!(non_uniform.is_none(), "non-uniform arrays are refused");
}

#[test]
fn af_desc_object_array_bounds_values() {
    let mut bytes = Vec::new();
    push_utf16(&mut bytes, "Root\0");
    push_code_key(&mut bytes, b"root");
    push_u32(&mut bytes, 1);
    push_code_key(&mut bytes, b"obar");
    bytes.extend_from_slice(b"ObAr");
    push_u32(&mut bytes, 2_000_000);
    let failure = read_action_descriptor(&bytes, true).expect_err("object count is capped");
    assert!(
        failure.message.contains("object-array"),
        "{}",
        failure.message
    );

    let mut bytes = Vec::new();
    push_utf16(&mut bytes, "Root\0");
    push_code_key(&mut bytes, b"root");
    push_u32(&mut bytes, 1);
    push_code_key(&mut bytes, b"obar");
    bytes.extend_from_slice(b"ObAr");
    push_u32(&mut bytes, 1_000_000);
    push_utf16(&mut bytes, "Pnt \0");
    push_code_key(&mut bytes, b"Pnt ");
    push_u32(&mut bytes, 2);
    bytes.extend_from_slice(&[0u8; 64]);
    let failure = read_action_descriptor(&bytes, true).expect_err("objects x items is capped");
    assert!(
        failure.message.contains("object-array"),
        "{}",
        failure.message
    );

    let mut bytes = Vec::new();
    push_utf16(&mut bytes, "Root\0");
    push_code_key(&mut bytes, b"root");
    push_u32(&mut bytes, 1);
    push_code_key(&mut bytes, b"obar");
    bytes.extend_from_slice(b"ObAr");
    push_u32(&mut bytes, 1_000_000);
    push_utf16(&mut bytes, "Pnt \0");
    push_code_key(&mut bytes, b"Pnt ");
    push_u32(&mut bytes, 1);
    bytes.extend_from_slice(&[0u8; 64]);
    let failure =
        read_action_descriptor(&bytes, true).expect_err("object values must fit the bytes");
    assert!(
        failure.message.contains("object-array"),
        "{}",
        failure.message
    );
}

#[test]
fn af_psp_trailing_bytes_are_refused() {
    let set = fixture_set();
    let mut bytes = write_psp(std::slice::from_ref(&set));
    bytes.extend_from_slice(b"junk");

    let failure = read_psp(&bytes).expect_err("undeclared trailing palette data is refused");
    assert_eq!(failure.partial_sets, vec![set]);
    assert!(failure.message.contains("trailing"), "{}", failure.message);
}

#[test]
fn af_psp_set_count_is_bounded_by_bytes() {
    let mut bytes = Vec::new();
    push_u32(&mut bytes, 16);
    push_u32(&mut bytes, 1_000_000);

    let failure = read_psp(&bytes).expect_err("declared set count must fit the file");
    assert!(
        failure.message.contains("palette set"),
        "{}",
        failure.message
    );
}

#[test]
fn af_atn_noncanonical_flags_are_normalized() {
    let mut bytes = Vec::new();
    push_u32(&mut bytes, 16);
    push_units(&mut bytes, &[]);
    bytes.push(2);
    push_u32(&mut bytes, 1);
    push_u16(&mut bytes, 0);
    bytes.push(2);
    bytes.push(2);
    push_u16(&mut bytes, 7);
    push_units(&mut bytes, &[]);
    bytes.push(2);
    push_u32(&mut bytes, 1);
    bytes.push(2);
    bytes.push(2);
    bytes.push(2);
    bytes.push(0);
    bytes.extend_from_slice(b"long");
    bytes.extend_from_slice(b"test");
    push_u32(&mut bytes, 0);
    push_i32(&mut bytes, 0);

    assert_eq!(bytes[8], 2);
    assert_eq!(bytes[15], 2);
    assert_eq!(bytes[16], 2);
    assert_eq!(bytes[23], 2);
    assert_eq!(bytes[28], 2);
    assert_eq!(bytes[29], 2);
    assert_eq!(bytes[30], 2);

    let decoded = read_atn(&bytes).expect("non-canonical flags read");
    assert!(decoded.set.expanded);
    assert!(decoded.set.actions[0].shift);
    assert!(decoded.set.actions[0].ctrl);
    assert!(decoded.set.actions[0].expanded);
    assert!(decoded.set.actions[0].steps[0].expanded);
    assert!(decoded.set.actions[0].steps[0].enabled);
    assert!(decoded.set.actions[0].steps[0].with_dialog);

    let written = write_atn(&decoded);
    assert_eq!(written[8], 1);
    assert_eq!(written[15], 1);
    assert_eq!(written[16], 1);
    assert_eq!(written[23], 1);
    assert_eq!(written[28], 1);
    assert_eq!(written[29], 1);
    assert_eq!(written[30], 1);
    assert_eq!(read_atn(&written).expect("normalized flags read"), decoded);
}

#[test]
fn real_atn_corpus_round_trips_when_present() {
    let Some(root) = std::env::var_os("PICTURA_ATN_CORPUS") else {
        return;
    };
    let mut paths = Vec::new();
    collect_corpus(&PathBuf::from(root), &mut paths);
    assert!(
        !paths.is_empty(),
        "PICTURA_ATN_CORPUS contains no .atn or .psp files"
    );

    for path in paths {
        let bytes = std::fs::read(&path).expect("corpus file is readable");
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("atn") => {
                let decoded = read_atn(&bytes)
                    .unwrap_or_else(|failure| panic!("{}: {failure}", path.display()));
                let written = write_atn(&decoded);
                assert_eq!(
                    written,
                    bytes,
                    "{} did not round-trip byte-for-byte",
                    path.display()
                );
                assert_eq!(read_atn(&written).unwrap(), decoded);
            }
            Some("psp") => {
                let sets = read_psp(&bytes)
                    .unwrap_or_else(|failure| panic!("{}: {failure}", path.display()));
                let written = write_psp(&sets);
                assert_eq!(
                    written,
                    bytes,
                    "{} did not round-trip byte-for-byte",
                    path.display()
                );
                assert_eq!(read_psp(&written).unwrap(), sets, "{}", path.display());
            }
            _ => {}
        }
    }
}

fn collect_corpus(path: &Path, paths: &mut Vec<PathBuf>) {
    if path.is_dir() {
        for entry in std::fs::read_dir(path).expect("corpus directory is readable") {
            collect_corpus(&entry.expect("corpus entry is readable").path(), paths);
        }
    } else if matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("atn") | Some("psp")
    ) {
        paths.push(path.to_path_buf());
    }
}
