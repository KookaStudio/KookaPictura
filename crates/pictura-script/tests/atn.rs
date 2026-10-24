//! Lossless `.atn` conversion tests: raw descriptors, names, app-native
//! payloads, and Adobe event disambiguation.

use pictura_codec::action_descriptor::{
    write_action_descriptor, ActionClass, ActionDescriptor, ActionKey, ActionReferenceItem,
    ActionValue, ReferenceForm, ReferenceValue,
};
use pictura_codec::action_file::{
    read_atn as codec_read_atn, write_atn as codec_write_atn, RawAction, RawActionSet, RawAtn,
    RawEventId, RawStep,
};
use pictura_script::atn::{read_atn, write_atn};
use pictura_script::{
    command::{self, CommandParams},
    Action, ActionLibrary, ActionSet, ActionStep, DialogOptions, EventKey, StepKind, ToolStroke,
};
use serde_json::json;

fn code(name: [u8; 4], value: ActionValue) -> (ActionKey, ActionValue) {
    (ActionKey::Code(name), value)
}

fn descriptor(name: &str, items: Vec<(ActionKey, ActionValue)>) -> ActionDescriptor {
    ActionDescriptor {
        name: name.to_string(),
        class: ActionClass {
            name: name.to_string(),
            id: ActionKey::Code(*b"null"),
        },
        items,
    }
}

fn json_item(value: serde_json::Value) -> (ActionKey, ActionValue) {
    (
        ActionKey::Code(*b"jsn "),
        ActionValue::Text(value.to_string()),
    )
}

fn raw_step(
    event: RawEventId,
    descriptor: Option<ActionDescriptor>,
    display_name: &str,
) -> RawStep {
    RawStep {
        expanded: true,
        enabled: true,
        with_dialog: false,
        dialog_options: 0,
        event_id: event,
        display_name: display_name.as_bytes().to_vec(),
        descriptor,
    }
}

fn raw_set(name: Vec<u16>, steps: Vec<RawStep>) -> RawActionSet {
    RawActionSet {
        name,
        expanded: true,
        actions: vec![RawAction {
            function_key: 0,
            shift: false,
            ctrl: false,
            color: 0,
            name: "Imported".encode_utf16().chain([0]).collect(),
            expanded: true,
            steps,
        }],
    }
}

fn raw_bytes(set: RawActionSet) -> Vec<u8> {
    codec_write_atn(&RawAtn {
        set,
        trailing: Vec::new(),
    })
}

#[test]
fn mapped_steps_reemit_duplicate_unknown_and_unit_items() {
    let step = raw_step(
        RawEventId::Long(*b"GsnB"),
        Some(descriptor(
            "Gaussian Blur",
            vec![
                code(
                    *b"Rds ",
                    ActionValue::UnitFloat {
                        unit: *b"#Pxl",
                        value: 4.0,
                    },
                ),
                code(
                    *b"Rds ",
                    ActionValue::UnitFloat {
                        unit: *b"#Pxl",
                        value: 8.0,
                    },
                ),
                code(*b"Xtra", ActionValue::Text("keep me".into())),
                json_item(json!({ "radius": 8.0 })),
            ],
        )),
        "Gaussian Blur",
    );
    let bytes = raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![step],
    ));

    let set = read_atn(&bytes).expect("decode");
    let step = &set.actions[0].steps[0];
    assert!(matches!(
        &step.kind,
        StepKind::Command { command, .. } if command.as_str() == "filter.blur.gaussian.blur"
    ));
    assert!(step.imported.is_some(), "imported form must be kept");

    let reencoded = write_atn(&set);
    assert_eq!(bytes, reencoded, "unedited step must re-emit byte-for-byte");
}

#[test]
fn stop_play_and_menu_item_keep_their_imported_descriptors() {
    let extra = code(*b"Xtra", ActionValue::Text("unknown".into()));
    let steps = vec![
        {
            let mut item = raw_step(
                RawEventId::Long(*b"Stop"),
                Some(descriptor(
                    "Stop",
                    vec![
                        code(*b"Msge", ActionValue::Text("hello".into())),
                        code(*b"Cntn", ActionValue::Boolean(true)),
                        extra.clone(),
                    ],
                )),
                "Stop",
            );
            item.dialog_options = 4;
            item
        },
        raw_step(
            RawEventId::Long(*b"Ply "),
            Some(descriptor(
                "Play",
                vec![
                    code(*b"Nm  ", ActionValue::Text("Nested".into())),
                    extra.clone(),
                ],
            )),
            "Play",
        ),
        raw_step(
            RawEventId::Long(*b"slct"),
            Some(descriptor(
                "Insert Menu Item",
                vec![code(*b"null", ActionValue::Text("edit.copy".into())), extra],
            )),
            "menu",
        ),
    ];
    let bytes = raw_bytes(raw_set("Set".encode_utf16().chain([0]).collect(), steps));
    let set = read_atn(&bytes).expect("decode");
    assert!(matches!(
        set.actions[0].steps[0].kind,
        StepKind::Stop { .. }
    ));
    assert!(matches!(
        set.actions[0].steps[1].kind,
        StepKind::Play { .. }
    ));
    assert!(matches!(
        set.actions[0].steps[2].kind,
        StepKind::MenuItem { .. }
    ));
    let normalized = write_atn(&set);
    let second = read_atn(&normalized).expect("decode normalized");
    assert_eq!(normalized, write_atn(&second));
    assert_eq!(
        second.actions[0].steps[2].dialog_options,
        DialogOptions::SPECIFIC
    );
}

#[test]
fn unknown_steps_round_trip_with_their_descriptor() {
    let descriptor = descriptor(
        "Mystery",
        vec![code(*b"Data", ActionValue::RawData(vec![1, 2, 3]))],
    );
    let step = ActionStep {
        flags: Default::default(),
        dialog_options: DialogOptions::OPTIONAL,
        display_name: "Mystery".into(),
        kind: StepKind::Unknown {
            event: EventKey::text("AdobeMystery"),
            descriptor: Some(write_action_descriptor(&descriptor, true)),
        },
        imported: None,
    };
    let mut set = ActionSet::new("Set");
    let mut action = Action::new("Action");
    action.steps.push(step.clone());
    set.actions.push(action);
    let library = ActionLibrary::from_sets(vec![set]);
    let bytes = write_atn(&library.sets()[0]);
    let loaded = read_atn(&bytes).expect("load");
    let loaded_step = &loaded.actions[0].steps[0];
    assert!(matches!(
        &loaded_step.kind,
        StepKind::Unknown { event, .. } if event.display() == "AdobeMystery"
    ));
    let reencoded = write_atn(&loaded);
    let second = pictura_codec::action_file::read_atn(&reencoded).expect("codec");
    let raw_descriptor = second.set.actions[0].steps[0]
        .descriptor
        .as_ref()
        .expect("descriptor");
    assert_eq!(raw_descriptor.items.len(), 1);
    assert_eq!(raw_descriptor.items[0].0, ActionKey::Code(*b"Data"));
    assert_eq!(loaded_step.kind, step.kind);
}

#[test]
fn imported_names_and_display_names_are_byte_preserved() {
    let mut step = raw_step(RawEventId::Text(vec![0xFF]), None, "ignored");
    step.display_name = vec![0xFF, 0xFE];
    let set = RawActionSet {
        name: vec![0xD800, 0x0041],
        expanded: false,
        actions: vec![RawAction {
            function_key: 0,
            shift: false,
            ctrl: false,
            color: 0,
            name: vec![0x0042],
            expanded: true,
            steps: vec![step],
        }],
    };
    let bytes = raw_bytes(set);
    let decoded = read_atn(&bytes).expect("decode");
    assert_eq!(decoded.raw_name.as_deref(), Some(&[0xD800, 0x0041][..]));
    assert_eq!(decoded.actions[0].raw_name.as_deref(), Some(&[0x0042][..]));
    assert_eq!(write_atn(&decoded), bytes);

    let raw = codec_read_atn(&write_atn(&decoded)).expect("codec");
    assert_eq!(raw.set.name, vec![0xD800, 0x0041]);
    assert!(!raw.set.name.contains(&0), "no-NUL names must stay no-NUL");
    assert_eq!(raw.set.actions[0].name, vec![0x0042]);
    assert_eq!(raw.set.actions[0].steps[0].display_name, vec![0xFF, 0xFE]);
}

#[test]
fn unknown_conditional_json_is_an_unknown_step() {
    let step = raw_step(
        RawEventId::Text(b"pictura.conditional".to_vec()),
        Some(descriptor(
            "pictura.conditional",
            vec![json_item(json!({
                "v": 1,
                "condition": "DocumentIsPortrait",
                "then": "A",
                "else": null,
            }))],
        )),
        "Conditional",
    );
    let bytes = raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![step],
    ));
    let set = read_atn(&bytes).expect("decode");
    assert!(matches!(
        &set.actions[0].steps[0].kind,
        StepKind::Unknown { event, .. } if event.display() == "pictura.conditional"
    ));
    assert_eq!(write_atn(&set), bytes);

    let missing = raw_step(
        RawEventId::Text(b"pictura.conditional".to_vec()),
        Some(descriptor(
            "pictura.conditional",
            vec![json_item(json!({ "v": 1 }))],
        )),
        "Conditional",
    );
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![missing],
    )))
    .expect("decode missing");
    assert!(matches!(
        set.actions[0].steps[0].kind,
        StepKind::Unknown { .. }
    ));
}

#[test]
fn tool_strokes_enforce_the_version_and_payload_bound() {
    let valid = ToolStroke::new("brush", CommandParams::new(), vec![]);
    let valid_step = raw_step(
        RawEventId::Text(b"pictura.toolStroke".to_vec()),
        Some(descriptor(
            "pictura.toolStroke",
            vec![json_item(serde_json::to_value(valid).expect("stroke"))],
        )),
        "Stroke",
    );
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![valid_step],
    )))
    .expect("decode valid");
    assert!(matches!(
        set.actions[0].steps[0].kind,
        StepKind::ToolStroke { .. }
    ));

    for payload in [
        json!({
            "version": 0,
            "tool": "brush",
            "settings": {},
            "samples": [],
        }),
        json!({
            "version": ToolStroke::VERSION,
            "tool": "brush",
            "settings": { "blob": "x".repeat(pictura_script::recorder::MAX_TOOL_PAYLOAD_BYTES) },
            "samples": [],
        }),
    ] {
        let step = raw_step(
            RawEventId::Text(b"pictura.toolStroke".to_vec()),
            Some(descriptor("pictura.toolStroke", vec![json_item(payload)])),
            "Stroke",
        );
        let bytes = raw_bytes(raw_set(
            "Set".encode_utf16().chain([0]).collect(),
            vec![step],
        ));
        let set = read_atn(&bytes).expect("decode invalid");
        assert!(matches!(
            &set.actions[0].steps[0].kind,
            StepKind::Unknown { event, descriptor }
                if event.display() == "pictura.toolStroke" && descriptor.is_some()
        ));
        assert_eq!(write_atn(&set), bytes, "rejected payload stays raw");
    }
}

#[test]
fn adobe_when_parameter_is_written_and_used_for_mapping() {
    let mut set = ActionSet::new("Set");
    let mut action = Action::new("Mode");
    action.steps.push(ActionStep::command(
        "image.mode.grayscale",
        CommandParams::new(),
    ));
    set.actions.push(action);
    let bytes = write_atn(&set);
    let raw = codec_read_atn(&bytes).expect("codec");
    let items = &raw.set.actions[0].steps[0]
        .descriptor
        .as_ref()
        .expect("descriptor")
        .items;
    assert!(items.iter().any(|(key, value)| {
        key == &ActionKey::Code(*b"Md  ")
            && value
                == &ActionValue::Enumerated {
                    kind: ActionKey::Code(*b"Md  "),
                    value: ActionKey::Code(*b"Grys"),
                }
    }));

    let loaded = read_atn(&bytes).expect("decode");
    let step = &loaded.actions[0].steps[0];
    assert!(matches!(
        &step.kind,
        StepKind::Command { command, params }
            if command.as_str() == "image.mode.grayscale"
                && params.get_str("mode") == Some("grayscale")
    ));

    let imported = raw_step(
        RawEventId::Long(*b"CnvM"),
        Some(descriptor(
            "mode",
            vec![
                code(
                    *b"Md  ",
                    ActionValue::Enumerated {
                        kind: ActionKey::Code(*b"Md  "),
                        value: ActionKey::Code(*b"Grys"),
                    },
                ),
                json_item(json!({})),
            ],
        )),
        "Grayscale",
    );
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![imported],
    )))
    .expect("decode imported mode");
    assert!(matches!(
        &set.actions[0].steps[0].kind,
        StepKind::Command { command, .. } if command.as_str() == "image.mode.grayscale"
    ));
}

#[test]
fn dialog_options_are_validated_and_menu_items_are_specific() {
    let mut step = raw_step(
        RawEventId::Long(*b"slct"),
        Some(descriptor(
            "menu",
            vec![code(*b"null", ActionValue::Text("edit.copy".into()))],
        )),
        "menu",
    );
    step.dialog_options = 0;
    step.with_dialog = true;
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![step],
    )))
    .expect("decode menu");
    let decoded = &set.actions[0].steps[0];
    assert!(matches!(decoded.kind, StepKind::MenuItem { .. }));
    assert_eq!(decoded.dialog_options, DialogOptions::SPECIFIC);
    assert!(!decoded.flags.with_dialog);

    let mut invalid = raw_step(
        RawEventId::Long(*b"Stop"),
        Some(descriptor("Stop", vec![])),
        "Stop",
    );
    invalid.dialog_options = 9;
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![invalid],
    )))
    .expect("decode invalid");
    assert!(matches!(
        set.actions[0].steps[0].kind,
        StepKind::Unknown { .. }
    ));
}

#[test]
fn rotate_and_flip_events_map_by_angle_axis_and_family() {
    let angle = |value: f64| CommandParams::new().with("angle", value);
    assert_eq!(
        command::command_id_for_event("Rtte", &angle(180.0)),
        Some("image.rotate180")
    );
    assert_eq!(
        command::command_id_for_event("Rtte", &angle(90.0)),
        Some("image.rotate90cw")
    );
    assert_eq!(
        command::command_id_for_event("Rtte", &angle(-90.0)),
        Some("image.rotate90ccw")
    );
    assert_eq!(
        command::command_id_for_event("Rtte", &angle(270.0)),
        Some("image.rotate90ccw")
    );
    assert_eq!(
        command::command_id_for_event("Rtte", &angle(45.0)),
        Some("image.rotate.arbitrary")
    );
    let layer = CommandParams::new().with("angle", 90.0).with("layer", true);
    assert_eq!(
        command::command_id_for_event("Rtte", &layer),
        Some("edit.transform.rotate90cw")
    );
    let layer180 = CommandParams::new()
        .with("angle", 180.0)
        .with("layer", true);
    assert_eq!(
        command::command_id_for_event("Rtte", &layer180),
        Some("edit.transform.rotate180")
    );

    let horizontal = CommandParams::new().with("axis", "horizontal");
    assert_eq!(
        command::command_id_for_event("Flp ", &horizontal),
        Some("image.flipHorizontal")
    );
    let vertical = CommandParams::new().with("axis", "vertical");
    assert_eq!(
        command::command_id_for_event("Flp ", &vertical),
        Some("image.flipVertical")
    );
    let layer_vertical = CommandParams::new()
        .with("axis", "Vrtc")
        .with("layer", true);
    assert_eq!(
        command::command_id_for_event("Flp ", &layer_vertical),
        Some("edit.transform.flipVertical")
    );
    assert_eq!(
        command::command_id_for_event("Flp ", &CommandParams::new()),
        None
    );
}

#[test]
fn a_layer_reference_selects_the_transform_family() {
    let reference = ActionValue::Reference(vec![ActionReferenceItem {
        form: ReferenceForm::Class,
        class: ActionClass {
            name: "layer".into(),
            id: ActionKey::Code(*b"Lyr "),
        },
        key: None,
        value: ReferenceValue::None,
    }]);
    let step = raw_step(
        RawEventId::Long(*b"Rtte"),
        Some(descriptor(
            "rotate",
            vec![
                (ActionKey::Code(*b"null"), reference),
                code(
                    *b"Angl",
                    ActionValue::UnitFloat {
                        unit: *b"#Ang",
                        value: 90.0,
                    },
                ),
            ],
        )),
        "Rotate 90° Clockwise",
    );
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![step],
    )))
    .expect("decode");
    assert!(matches!(
        &set.actions[0].steps[0].kind,
        StepKind::Command { command, .. } if command.as_str() == "edit.transform.rotate90cw"
    ));
}

#[test]
fn editing_a_parameter_clears_the_imported_descriptor() {
    let step = raw_step(
        RawEventId::Long(*b"GsnB"),
        Some(descriptor(
            "Gaussian Blur",
            vec![
                code(
                    *b"Rds ",
                    ActionValue::UnitFloat {
                        unit: *b"#Pxl",
                        value: 4.0,
                    },
                ),
                code(*b"Xtra", ActionValue::Text("drop me when edited".into())),
                json_item(json!({ "radius": 4.0 })),
            ],
        )),
        "Gaussian Blur",
    );
    let set = read_atn(&raw_bytes(raw_set(
        "Set".encode_utf16().chain([0]).collect(),
        vec![step],
    )))
    .expect("decode");
    let mut library = ActionLibrary::from_sets(vec![set]);
    assert!(library.set_step_params(0, 0, 0, CommandParams::new().with("radius", 2.0)));
    let bytes = write_atn(&library.sets()[0]);
    let raw = codec_read_atn(&bytes).expect("codec");
    let items = &raw.set.actions[0].steps[0]
        .descriptor
        .as_ref()
        .expect("descriptor")
        .items;
    assert!(items.iter().any(|(key, value)| {
        key == &ActionKey::Code(*b"Rds ")
            && value
                == &ActionValue::UnitFloat {
                    unit: *b"#Pxl",
                    value: 2.0,
                }
    }));
    assert!(!items
        .iter()
        .any(|(key, _)| key == &ActionKey::Code(*b"Xtra")));
}

#[test]
fn renaming_an_imported_name_replaces_stale_units() {
    let mut library = ActionLibrary::from_sets(vec![read_atn(&raw_bytes(raw_set(
        vec![0x0041],
        vec![raw_step(RawEventId::Text(vec![0x41]), None, "Step")],
    )))
    .expect("decode")]);
    assert!(library.rename_set(0, "New Set"));
    assert!(library
        .rename_action(0, 0, "New Action")
        .expect("rename action"));
    let raw = codec_read_atn(&library.set_bytes(0).expect("write")).expect("codec");
    assert_eq!(
        raw.set.name,
        "New Set".encode_utf16().chain([0]).collect::<Vec<_>>()
    );
    assert_eq!(
        raw.set.actions[0].name,
        "New Action".encode_utf16().chain([0]).collect::<Vec<_>>()
    );
}
