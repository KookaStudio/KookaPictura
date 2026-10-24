//! Engine tests for the `pictura-script` rows of `test-plan.md`.

use pictura_script::{
    command::{self, CommandId, CommandParams},
    Action, ActionColor, ActionError, ActionLibrary, ActionSet, ActionStep, Condition,
    DialogOptions, DocumentFacts, EventKey, FunctionKey, PlaybackCommand, PlaybackOptions,
    PlaybackSelection, Player, Recorder, ResetMode, Selection, StepFlags, StepKind,
};

fn command_step(id: &str) -> ActionStep {
    ActionStep::command(id, CommandParams::new())
}

fn action(name: &str, steps: Vec<ActionStep>) -> Action {
    let mut action = Action::new(name);
    action.steps = steps;
    action
}

fn library(set: ActionSet) -> ActionLibrary {
    ActionLibrary::from_sets(vec![set])
}

fn facts(square: bool, present: bool) -> DocumentFacts {
    DocumentFacts {
        present,
        square,
        mode: "rgb".into(),
        has_selection: false,
    }
}

fn next_id(player: &mut Player) -> Option<String> {
    match player.next().expect("playback step") {
        PlaybackCommand::Execute { command, .. } => Some(command.to_string()),
        PlaybackCommand::Finished => None,
        other => panic!("unexpected playback instruction: {other:?}"),
    }
}

#[test]
fn ae_model_roundtrip() {
    let mut set = ActionSet::new("Ünïcode set");
    set.expanded = false;
    let mut first = Action::new("First");
    first.function_key = FunctionKey::new(5, true, false);
    first.color = ActionColor::BLUE;
    first.expanded = false;
    first.steps.push(command_step("edit.cut"));
    let mut second = command_step("filter.blur.gaussian.blur");
    second.flags = StepFlags {
        expanded: false,
        enabled: false,
        with_dialog: true,
    };
    second.dialog_options = DialogOptions::SPECIFIC;
    first.steps.push(second);
    let mut nested = Action::new("Nested");
    nested.steps.push(command_step("layer.flatten.image"));
    set.actions = vec![first, nested];

    let source = library(set.clone());
    let bytes = source.save_bytes();
    let mut loaded = ActionLibrary::empty();
    loaded.load_bytes("test.psp", &bytes).expect("load");
    assert_eq!(source.sets(), loaded.sets());
}

#[test]
fn every_step_kind_round_trips_through_the_palette() {
    let mut action = Action::new("Steps");
    action.steps = vec![
        command_step("edit.cut"),
        ActionStep {
            flags: StepFlags::default(),
            dialog_options: DialogOptions::OPTIONAL,
            display_name: "Stop".into(),
            kind: StepKind::Stop {
                message: "Paused".into(),
                allow_continue: true,
            },
            imported: None,
        },
        ActionStep {
            flags: StepFlags::default(),
            dialog_options: DialogOptions::SPECIFIC,
            display_name: "Menu".into(),
            kind: StepKind::MenuItem {
                command: CommandId::new("edit.copy"),
                params: CommandParams::new().with("layer", 2),
            },
            imported: None,
        },
        ActionStep {
            flags: StepFlags::default(),
            dialog_options: DialogOptions::OPTIONAL,
            display_name: "Path".into(),
            kind: StepKind::InsertPath {
                path: "Work Path".into(),
            },
            imported: None,
        },
        ActionStep {
            flags: StepFlags::default(),
            dialog_options: DialogOptions::OPTIONAL,
            display_name: "Play".into(),
            kind: StepKind::Play {
                action: "Steps".into(),
            },
            imported: None,
        },
        ActionStep {
            flags: StepFlags::default(),
            dialog_options: DialogOptions::OPTIONAL,
            display_name: "Conditional".into(),
            kind: StepKind::Conditional {
                condition: Condition::DocumentHasSelection,
                then_action: Some("Steps".into()),
                else_action: None,
            },
            imported: None,
        },
        ActionStep {
            flags: StepFlags::default(),
            dialog_options: DialogOptions::OPTIONAL,
            display_name: "Stroke".into(),
            kind: StepKind::ToolStroke {
                stroke: pictura_script::ToolStroke::new(
                    "brush",
                    CommandParams::new().with("size", 12),
                    vec![pictura_script::ToolSample {
                        x: 1.0,
                        y: 2.0,
                        pressure: 0.5,
                        time: 0.0,
                    }],
                ),
            },
            imported: None,
        },
    ];
    let mut set = ActionSet::new("Kinds");
    set.actions.push(action);
    let source = library(set);
    let bytes = source.save_bytes();
    let mut loaded = ActionLibrary::empty();
    loaded.load_bytes("kinds.psp", &bytes).expect("load");
    assert_eq!(source.sets(), loaded.sets());
}

#[test]
fn ae_model_empty() {
    let mut set = ActionSet::new("Empty");
    set.actions.push(Action::new("Nothing"));
    let library = library(set.clone());
    let bytes = library.save_bytes();
    let mut loaded = ActionLibrary::empty();
    loaded.load_bytes("empty.psp", &bytes).expect("load empty");
    assert_eq!(loaded.sets(), library.sets());

    let mut player = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &DocumentFacts::default(),
        PlaybackOptions::default(),
    )
    .expect("start empty");
    assert_eq!(player.next().expect("finished"), PlaybackCommand::Finished);
}

#[test]
fn ae_load_failure_keeps_library() {
    let mut library = ActionLibrary::with_default();
    let before = library.sets().to_vec();
    let error = library
        .load_bytes("malformed.atn", &[0, 0, 0, 12, 0])
        .expect_err("malformed load must fail");
    assert_eq!(library.sets(), before.as_slice());
    match error {
        ActionError::Load { source_name, .. } => assert_eq!(source_name, "malformed.atn"),
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn ae_record_coverage() {
    let error = command::coverage_error_for(&["pictura.test.unclassified"])
        .expect_err("unclassified id must fail");
    assert!(error.to_string().contains("pictura.test.unclassified"));
    assert!(command::coverage_gaps().is_empty());
}

#[test]
fn ae_record_lifecycle() {
    let mut library = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![Action::new("Record")],
        trailing: Vec::new(),
        raw_name: None,
    });
    let mut recorder = Recorder::new();
    recorder
        .start(&library, Selection::Action { set: 0, action: 0 })
        .expect("start");
    assert!(recorder
        .record(
            &mut library,
            &CommandId::new("edit.cut"),
            &CommandParams::new()
        )
        .expect("record"));
    recorder.stop();
    recorder
        .start(&library, Selection::Action { set: 0, action: 0 })
        .expect("resume");
    assert!(recorder
        .record(
            &mut library,
            &CommandId::new("edit.copy"),
            &CommandParams::new()
        )
        .expect("record"));

    let steps = &library.action(0, 0).expect("action").steps;
    assert_eq!(steps.len(), 2);
    assert!(matches!(
        &steps[0].kind,
        StepKind::Command { command, .. } if command.as_str() == "edit.cut"
    ));
    assert!(matches!(
        &steps[1].kind,
        StepKind::Command { command, .. } if command.as_str() == "edit.copy"
    ));
}

#[test]
fn ae_play_order() {
    let mut second = command_step("edit.copy");
    second.flags.enabled = false;
    let set = ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![action(
            "Order",
            vec![
                command_step("edit.cut"),
                second,
                command_step("edit.paste"),
                command_step("edit.clear"),
            ],
        )],
        trailing: Vec::new(),
        raw_name: None,
    };
    let library = library(set);
    let mut player = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &DocumentFacts::default(),
        PlaybackOptions::default(),
    )
    .expect("play");
    let ids: Vec<String> = std::iter::from_fn(|| next_id(&mut player)).collect();
    assert_eq!(
        ids.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["edit.cut", "edit.paste", "edit.clear"]
    );
}

#[test]
fn ae_play_recursion() {
    let set = ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![
            action(
                "A",
                vec![ActionStep {
                    flags: StepFlags::default(),
                    dialog_options: DialogOptions::OPTIONAL,
                    display_name: "Play B".into(),
                    kind: StepKind::Play { action: "B".into() },
                    imported: None,
                }],
            ),
            action(
                "B",
                vec![ActionStep {
                    flags: StepFlags::default(),
                    dialog_options: DialogOptions::OPTIONAL,
                    display_name: "Play A".into(),
                    kind: StepKind::Play { action: "A".into() },
                    imported: None,
                }],
            ),
        ],
        trailing: Vec::new(),
        raw_name: None,
    };
    let library = library(set);
    let mut player = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &DocumentFacts::default(),
        PlaybackOptions::default(),
    )
    .expect("start");
    let error = player.next().expect_err("cycle must abort");
    assert!(matches!(error, ActionError::RecursionLimit));
}

#[test]
fn ae_play_error() {
    let set = ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![action(
            "Broken",
            vec![ActionStep {
                flags: StepFlags::default(),
                dialog_options: DialogOptions::OPTIONAL,
                display_name: "Play missing".into(),
                kind: StepKind::Play {
                    action: "Missing".into(),
                },
                imported: None,
            }],
        )],
        trailing: Vec::new(),
        raw_name: None,
    };
    let library = library(set);
    let mut player = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &DocumentFacts::default(),
        PlaybackOptions::default(),
    )
    .expect("start");
    let error = player
        .next()
        .expect_err("missing action must stop playback");
    match error {
        ActionError::Playback { step, message } => {
            assert_eq!(step, 0);
            assert!(message.contains("Missing"));
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn ae_unsupported_play() {
    let set = ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![action(
            "Imported",
            vec![ActionStep {
                flags: StepFlags::default(),
                dialog_options: DialogOptions::OPTIONAL,
                display_name: "Unknown".into(),
                kind: StepKind::Unknown {
                    event: EventKey::text("AdobeNope"),
                    descriptor: None,
                },
                imported: None,
            }],
        )],
        trailing: Vec::new(),
        raw_name: None,
    };
    let library = library(set);
    let mut player = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &DocumentFacts::default(),
        PlaybackOptions::default(),
    )
    .expect("start");
    let error = player.next().expect_err("unknown event must stop playback");
    match error {
        ActionError::UnsupportedEvent { event, step } => {
            assert_eq!(event, "AdobeNope");
            assert_eq!(step, 0);
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn ae_reset_replace() {
    let mut library = ActionLibrary::empty();
    library.create_set("Other");
    library.reset(ResetMode::Replace);
    assert_eq!(library.sets().len(), 1);
    assert_eq!(library.sets()[0].name, "Default Actions");
}

#[test]
fn ae_reset_append() {
    let mut library = ActionLibrary::empty();
    library.reset(ResetMode::Append);
    library.reset(ResetMode::Append);
    let names: Vec<&str> = library.sets().iter().map(|set| set.name.as_str()).collect();
    assert_eq!(names, vec!["Default Actions", "Default Actions 2"]);

    let mut existing = ActionLibrary::empty();
    existing.create_set("Mine");
    existing.reset(ResetMode::Append);
    let names: Vec<&str> = existing
        .sets()
        .iter()
        .map(|set| set.name.as_str())
        .collect();
    assert_eq!(names, vec!["Mine", "Default Actions"]);
}

#[test]
fn ae_clear_all() {
    let mut library = ActionLibrary::with_default();
    library.clear();
    assert!(library.sets().is_empty());
}

#[test]
fn ae_default_provenance() {
    let set = ActionLibrary::default_set();
    let names: Vec<&str> = set
        .actions
        .iter()
        .map(|action| action.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "Duplicate Layer",
            "Flatten Image",
            "Grayscale",
            "Resize to 1000 px"
        ]
    );
    assert!(names.iter().all(|name| !name.contains("Adobe")));
    let colors: std::collections::HashSet<u16> =
        set.actions.iter().map(|action| action.color.0).collect();
    assert_eq!(colors.len(), 4, "each default action needs its own color");
    let resize = &set.actions[3].steps[0];
    assert!(matches!(
        &resize.kind,
        StepKind::Command { command, params }
            if command.as_str() == "image.image.size"
                && params.get("width").and_then(serde_json::Value::as_i64) == Some(1000)
                && params
                    .get("constrainProportions")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
                && params.get_str("resample") == Some("bicubic")
    ));
}

fn conditional_step(
    condition: Condition,
    then_action: Option<&str>,
    else_action: Option<&str>,
) -> ActionStep {
    ActionStep {
        flags: StepFlags::default(),
        dialog_options: DialogOptions::OPTIONAL,
        display_name: "Conditional".into(),
        kind: StepKind::Conditional {
            condition,
            then_action: then_action.map(ToString::to_string),
            else_action: else_action.map(ToString::to_string),
        },
        imported: None,
    }
}

fn branch_library(then_action: Option<&str>, else_action: Option<&str>) -> ActionLibrary {
    let set = ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![
            action(
                "Choose",
                vec![conditional_step(
                    Condition::DocumentIsSquare,
                    then_action,
                    else_action,
                )],
            ),
            action("Square", vec![command_step("edit.cut")]),
            action("Wide", vec![command_step("edit.copy")]),
        ],
        trailing: Vec::new(),
        raw_name: None,
    };
    library(set)
}

#[test]
fn ae_conditional_branch() {
    let library = branch_library(Some("Square"), Some("Wide"));
    let mut square = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &facts(true, true),
        PlaybackOptions::default(),
    )
    .expect("square");
    assert_eq!(next_id(&mut square).as_deref(), Some("edit.cut"));
    assert_eq!(next_id(&mut square), None);

    let mut wide = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &facts(false, true),
        PlaybackOptions::default(),
    )
    .expect("wide");
    assert_eq!(next_id(&mut wide).as_deref(), Some("edit.copy"));
    assert_eq!(next_id(&mut wide), None);
}

#[test]
fn ae_conditional_invalid_play() {
    let library = branch_library(None, None);
    let mut player = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &facts(true, true),
        PlaybackOptions::default(),
    )
    .expect("start");
    let error = player.next().expect_err("both empty");
    assert!(error.to_string().contains("conditional"));
}

#[test]
fn ae_conditional_move_guard() {
    let mut library = branch_library(Some("Source"), None);
    library.sets_mut()[0].actions[1].name = "Source".into();
    library.sets_mut()[0].actions[2].name = "Other".into();
    library.create_set("Destination");

    let index = library.sets()[0]
        .action_index("Source")
        .expect("source action");
    assert!(matches!(
        library.delete_action(0, index),
        Err(ActionError::ActionReferenced { .. })
    ));
    assert!(matches!(
        library.move_action(0, index, 1, 0),
        Err(ActionError::ActionReferenced { .. })
    ));
    assert!(library.sets()[0].action("Source").is_some());
    let conditional = &library.sets()[0].actions[0].steps[0];
    assert!(matches!(
        &conditional.kind,
        StepKind::Conditional { then_action: Some(name), .. } if name == "Source"
    ));
}

#[test]
fn ae_conditional_no_document() {
    let library = branch_library(Some("Square"), None);
    let mut player = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &facts(true, false),
        PlaybackOptions::default(),
    )
    .expect("start");
    let error = player.next().expect_err("no document");
    assert!(matches!(error, ActionError::NoDocument));
}

#[test]
fn ae_conditional_recursion() {
    let set = ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![
            action(
                "A",
                vec![conditional_step(
                    Condition::DocumentHasSelection,
                    Some("B"),
                    None,
                )],
            ),
            action(
                "B",
                vec![conditional_step(
                    Condition::DocumentHasSelection,
                    Some("A"),
                    None,
                )],
            ),
        ],
        trailing: Vec::new(),
        raw_name: None,
    };
    let library = library(set);
    let mut player = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &DocumentFacts {
            present: true,
            square: true,
            mode: "rgb".into(),
            has_selection: true,
        },
        PlaybackOptions::default(),
    )
    .expect("start");
    let error = player.next().expect_err("conditional cycle");
    assert!(matches!(error, ActionError::RecursionLimit));
}

#[test]
fn function_keys_follow_platform_rules() {
    let mut library = branch_library(Some("Square"), None);
    let first = &mut library.sets_mut()[0].actions[0];
    first.function_key = FunctionKey::new(5, false, false);
    assert!(FunctionKey::new(1, false, false).is_allowed_on(pictura_script::Platform::Linux));
    assert!(!FunctionKey::new(1, false, false).is_allowed_on(pictura_script::Platform::Windows));
    assert!(!FunctionKey::new(4, false, true).is_allowed_on(pictura_script::Platform::Windows));
    assert!(FunctionKey::new(5, false, false).is_allowed_on(pictura_script::Platform::Windows));
}

#[test]
fn dialog_options_follow_the_five_values() {
    assert!(DialogOptions::REQUIRED.opens_dialog(false));
    assert!(!DialogOptions::NONE.opens_dialog(true));
    assert!(DialogOptions::SPECIFIC.opens_dialog(false));
    assert!(DialogOptions::OPTIONAL.opens_dialog(true));
    assert_eq!(DialogOptions::STOP_CONTINUE.written(), 0);
    assert!(!DialogOptions::SPECIFIC.modal_shown(false));
    assert!(!DialogOptions::REQUIRED.modal_editable());
}

#[test]
fn tool_stroke_is_bounded_and_captures_settings() {
    let mut library = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![Action::new("Record")],
        trailing: Vec::new(),
        raw_name: None,
    });
    let mut recorder = Recorder::new();
    recorder
        .start(&library, Selection::Action { set: 0, action: 0 })
        .expect("start");
    let stroke = pictura_script::ToolStroke::new(
        "brush",
        CommandParams::new().with("size", 12),
        vec![pictura_script::ToolSample {
            x: 1.0,
            y: 2.0,
            pressure: 0.5,
            time: 0.0,
        }],
    );
    assert!(!recorder
        .record_tool_stroke(&mut library, false, stroke.clone())
        .expect("off"));
    assert!(recorder
        .record_tool_stroke(&mut library, true, stroke)
        .expect("record"));
    let step = &library.sets()[0].actions[0].steps[0];
    match &step.kind {
        StepKind::ToolStroke { stroke } => {
            assert_eq!(stroke.settings().get("size"), Some(&serde_json::json!(12)));
        }
        other => panic!("unexpected step: {other:?}"),
    }
}

fn stop_step(message: &str, allow_continue: bool) -> ActionStep {
    ActionStep {
        flags: StepFlags::default(),
        dialog_options: DialogOptions::OPTIONAL,
        display_name: "Stop".into(),
        kind: StepKind::Stop {
            message: message.into(),
            allow_continue,
        },
        imported: None,
    }
}

fn play_step(name: &str) -> ActionStep {
    ActionStep {
        flags: StepFlags::default(),
        dialog_options: DialogOptions::OPTIONAL,
        display_name: format!("Play {name}"),
        kind: StepKind::Play {
            action: name.into(),
        },
        imported: None,
    }
}

#[test]
fn player_stop_without_continue_is_terminal() {
    let library = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![action(
            "Stop",
            vec![stop_step("halt", false), command_step("edit.cut")],
        )],
        trailing: Vec::new(),
        raw_name: None,
    });
    let mut player = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &DocumentFacts::default(),
        PlaybackOptions::default(),
    )
    .expect("start");
    match player.next().expect("stop") {
        PlaybackCommand::Stop {
            allow_continue,
            message,
            ..
        } => {
            assert!(!allow_continue);
            assert_eq!(message, "halt");
        }
        other => panic!("unexpected instruction: {other:?}"),
    }
    assert_eq!(player.next().expect("stopped"), PlaybackCommand::Finished);
    assert_eq!(
        player.next().expect("still stopped"),
        PlaybackCommand::Finished
    );
    assert!(player.is_finished());
    assert_eq!(player.remaining(), 0);
}

#[test]
fn player_stop_with_continue_resumes() {
    let library = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![action(
            "Stop",
            vec![stop_step("pause", true), command_step("edit.cut")],
        )],
        trailing: Vec::new(),
        raw_name: None,
    });
    let mut player = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &DocumentFacts::default(),
        PlaybackOptions::default(),
    )
    .expect("start");
    assert!(matches!(
        player.next().expect("stop"),
        PlaybackCommand::Stop {
            allow_continue: true,
            ..
        }
    ));
    assert_eq!(player.next().expect("waiting"), PlaybackCommand::Waiting);
    player.continue_stop();
    assert_eq!(next_id(&mut player).as_deref(), Some("edit.cut"));
    assert_eq!(next_id(&mut player), None);
}

#[test]
fn player_error_after_a_step_is_terminal() {
    let library = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![action(
            "Broken",
            vec![
                command_step("edit.cut"),
                play_step("Missing"),
                command_step("edit.paste"),
            ],
        )],
        trailing: Vec::new(),
        raw_name: None,
    });
    let mut player = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &DocumentFacts::default(),
        PlaybackOptions::default(),
    )
    .expect("start");
    assert_eq!(next_id(&mut player).as_deref(), Some("edit.cut"));
    assert!(player.next().is_err());
    assert!(
        player.next().is_err(),
        "a failed playback must not run the next step"
    );
    assert!(player.is_finished());
}

#[test]
fn player_empty_conditional_does_not_fall_through() {
    let library = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![action(
            "Broken",
            vec![
                conditional_step(Condition::DocumentIsSquare, None, None),
                command_step("edit.cut"),
            ],
        )],
        trailing: Vec::new(),
        raw_name: None,
    });
    let mut player = Player::start(
        &library,
        PlaybackSelection::Action { set: 0, action: 0 },
        &facts(true, true),
        PlaybackOptions::default(),
    )
    .expect("start");
    assert!(player.next().is_err());
    assert!(
        player.next().is_err(),
        "a failed conditional must not fall through to the next command"
    );
}

#[test]
fn player_from_step_and_single_step() {
    let library = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![action(
            "Order",
            vec![
                command_step("edit.cut"),
                command_step("edit.copy"),
                command_step("edit.paste"),
            ],
        )],
        trailing: Vec::new(),
        raw_name: None,
    });
    let mut from = Player::start(
        &library,
        PlaybackSelection::FromStep {
            set: 0,
            action: 0,
            step: 1,
        },
        &DocumentFacts::default(),
        PlaybackOptions::default(),
    )
    .expect("from step");
    assert_eq!(next_id(&mut from).as_deref(), Some("edit.copy"));
    assert_eq!(next_id(&mut from).as_deref(), Some("edit.paste"));
    assert_eq!(next_id(&mut from), None);

    let mut single = Player::start(
        &library,
        PlaybackSelection::SingleStep {
            set: 0,
            action: 0,
            step: 1,
        },
        &DocumentFacts::default(),
        PlaybackOptions::default(),
    )
    .expect("single step");
    assert_eq!(next_id(&mut single).as_deref(), Some("edit.copy"));
    assert_eq!(next_id(&mut single), None);
}

#[test]
fn rename_updates_play_refs_and_rejects_duplicates() {
    let mut library = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![
            action("Caller", vec![play_step("Target")]),
            action("Target", vec![command_step("edit.cut")]),
        ],
        trailing: Vec::new(),
        raw_name: None,
    });
    library.rename_action(0, 1, "Renamed").expect("rename");
    let caller = &library.sets()[0].actions[0].steps[0];
    assert!(matches!(
        &caller.kind,
        StepKind::Play { action } if action == "Renamed"
    ));
    assert!(matches!(
        library.create_action(0, "Renamed"),
        Err(ActionError::DuplicateActionName { .. })
    ));
    assert!(matches!(
        library.rename_action(0, 0, "Renamed"),
        Err(ActionError::DuplicateActionName { .. })
    ));
}

#[test]
fn duplicates_clear_function_keys() {
    let mut library = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![
            {
                let mut first = action("First", vec![command_step("edit.cut")]);
                first.function_key = FunctionKey::new(5, false, false);
                first
            },
            action("Second", vec![command_step("edit.copy")]),
        ],
        trailing: Vec::new(),
        raw_name: None,
    });
    let copy_index = library.duplicate_action(0, 0).expect("duplicate action");
    assert_eq!(
        library.sets()[0].actions[copy_index].function_key,
        FunctionKey::NONE
    );
    let set_index = library.duplicate_set(0).expect("duplicate set");
    assert!(library.sets()[set_index]
        .actions
        .iter()
        .all(|action| action.function_key == FunctionKey::NONE));
}

#[test]
fn moves_accept_append_and_noop() {
    let mut sets =
        ActionLibrary::from_sets(vec![ActionSet::new("First"), ActionSet::new("Second")]);
    assert!(sets.move_set(0, 2));
    assert_eq!(sets.sets()[1].name, "First");
    assert!(sets.move_set(0, 0));

    let mut library = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![
            action(
                "A",
                vec![command_step("edit.cut"), command_step("edit.copy")],
            ),
            action("B", vec![command_step("edit.paste")]),
        ],
        trailing: Vec::new(),
        raw_name: None,
    });
    assert!(library.move_action(0, 0, 0, 2).expect("move"));
    assert_eq!(library.sets()[0].actions[1].name, "A");
    assert!(library.move_action(0, 0, 0, 0).expect("move"));

    assert!(library.move_step(0, 1, 0, 2));
    assert!(matches!(
        &library.sets()[0].actions[1].steps[0].kind,
        StepKind::Command { command, .. } if command.as_str() == "edit.copy"
    ));
    assert!(library.move_step(0, 1, 1, 1));
}

#[test]
fn move_step_to_moves_across_actions_and_sets() {
    let mut library = ActionLibrary::from_sets(vec![
        ActionSet {
            name: "Source".into(),
            expanded: true,
            actions: vec![action(
                "From",
                vec![command_step("edit.cut"), command_step("edit.copy")],
            )],
            trailing: Vec::new(),
            raw_name: None,
        },
        ActionSet {
            name: "Destination".into(),
            expanded: true,
            actions: vec![action("To", vec![command_step("edit.paste")])],
            trailing: Vec::new(),
            raw_name: None,
        },
    ]);
    assert!(library
        .move_step_to(0, 0, 0, 0, 0, 2)
        .expect("same action append"));
    assert!(library
        .move_step_to(0, 0, 1, 1, 0, 0)
        .expect("cross set insert"));
    let source = &library.sets()[0].actions[0].steps;
    assert_eq!(source.len(), 1);
    assert!(matches!(
        &source[0].kind,
        StepKind::Command { command, .. } if command.as_str() == "edit.copy"
    ));
    let destination = &library.sets()[1].actions[0].steps;
    assert_eq!(destination.len(), 2);
    assert!(matches!(
        &destination[0].kind,
        StepKind::Command { command, .. } if command.as_str() == "edit.cut"
    ));
    assert!(library
        .move_step_to(0, 0, 0, 1, 0, 2)
        .expect("append destination"));
    assert!(library.sets()[0].actions[0].steps.is_empty());
    let destination = &library.sets()[1].actions[0].steps;
    assert_eq!(destination.len(), 3);
    assert!(matches!(
        &destination[2].kind,
        StepKind::Command { command, .. } if command.as_str() == "edit.copy"
    ));
    assert!(library.move_step_to(1, 0, 1, 1, 0, 1).expect("noop"));
}

#[test]
fn cross_set_move_checks_conditional_branches() {
    let mut library = ActionLibrary::from_sets(vec![
        ActionSet {
            name: "Source".into(),
            expanded: true,
            actions: vec![
                action(
                    "Chooser",
                    vec![conditional_step(
                        Condition::DocumentIsSquare,
                        Some("Helper"),
                        None,
                    )],
                ),
                action("Helper", vec![command_step("edit.cut")]),
            ],
            trailing: Vec::new(),
            raw_name: None,
        },
        ActionSet {
            name: "Destination".into(),
            expanded: true,
            actions: vec![action("Other", vec![command_step("edit.copy")])],
            trailing: Vec::new(),
            raw_name: None,
        },
    ]);
    assert!(matches!(
        library.move_action(0, 0, 1, 0),
        Err(ActionError::ConditionalCrossSet)
    ));
    assert_eq!(library.sets()[0].actions.len(), 2);
}

#[test]
fn recorder_clears_a_target_that_moved_or_vanished() {
    let mut lib = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![
            action("Record", vec![command_step("edit.cut")]),
            action("Other", vec![command_step("edit.copy")]),
        ],
        trailing: Vec::new(),
        raw_name: None,
    });
    let mut recorder = Recorder::new();
    recorder
        .start(&lib, Selection::Action { set: 0, action: 0 })
        .expect("start");
    assert!(lib.move_action(0, 0, 0, 2).expect("move"));
    assert!(matches!(
        recorder.record(
            &mut lib,
            &CommandId::new("edit.paste"),
            &CommandParams::new()
        ),
        Err(ActionError::NoTarget)
    ));
    assert!(!recorder.is_recording());

    recorder
        .start(&lib, Selection::Action { set: 0, action: 1 })
        .expect("start again");
    assert!(lib.delete_action(0, 1).expect("delete"));
    assert!(matches!(
        recorder.record(
            &mut lib,
            &CommandId::new("edit.paste"),
            &CommandParams::new()
        ),
        Err(ActionError::NoTarget)
    ));
    assert!(!recorder.is_recording());

    let mut edited = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![action(
            "Record",
            vec![command_step("edit.cut"), command_step("edit.copy")],
        )],
        trailing: Vec::new(),
        raw_name: None,
    });
    recorder
        .start(
            &edited,
            Selection::Step {
                set: 0,
                action: 0,
                step: 0,
            },
        )
        .expect("start on step");
    assert!(edited.delete_step(0, 0, 0));
    assert!(matches!(
        recorder.record(
            &mut edited,
            &CommandId::new("edit.paste"),
            &CommandParams::new()
        ),
        Err(ActionError::NoTarget)
    ));
    assert!(!recorder.is_recording());
}

#[test]
fn insert_path_replaces_until_a_save_path() {
    let mut library = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![Action::new("Paths")],
        trailing: Vec::new(),
        raw_name: None,
    });
    assert_eq!(library.insert_path(0, 0, "one").expect("first"), 0);
    assert_eq!(library.insert_path(0, 0, "two").expect("replace"), 0);
    assert_eq!(library.sets()[0].actions[0].steps.len(), 1);
    assert!(matches!(
        &library.sets()[0].actions[0].steps[0].kind,
        StepKind::InsertPath { path } if path == "two"
    ));

    let mut save = command_step("file.save");
    save.display_name = "Save Path".into();
    library.add_step(0, 0, save).expect("save path");
    assert_eq!(library.insert_path(0, 0, "three").expect("append"), 2);
    assert_eq!(library.insert_path(0, 0, "four").expect("replace"), 2);
    assert_eq!(library.sets()[0].actions[0].steps.len(), 3);
    assert!(matches!(
        &library.sets()[0].actions[0].steps[2].kind,
        StepKind::InsertPath { path } if path == "four"
    ));
}

#[test]
fn recorded_commands_carry_their_dialog_mode() {
    let mut library = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![Action::new("Record")],
        trailing: Vec::new(),
        raw_name: None,
    });
    let mut recorder = Recorder::new();
    recorder
        .start(&library, Selection::Action { set: 0, action: 0 })
        .expect("start");
    for id in [
        "edit.fill",
        "layer.flatten.image",
        "filter.blur.gaussian.blur",
    ] {
        assert!(recorder
            .record(&mut library, &CommandId::new(id), &CommandParams::new())
            .expect("record"));
    }
    let steps = &library.sets()[0].actions[0].steps;
    assert_eq!(steps[0].dialog_options, DialogOptions::REQUIRED);
    assert!(steps[0].opens_dialog());
    assert_eq!(steps[1].dialog_options, DialogOptions::NONE);
    assert!(!steps[1].opens_dialog());
    assert_eq!(steps[2].dialog_options, DialogOptions::OPTIONAL);
    assert!(!steps[2].opens_dialog());
}

#[test]
fn oversized_tool_strokes_are_refused() {
    let mut library = library(ActionSet {
        name: "Set".into(),
        expanded: true,
        actions: vec![Action::new("Record")],
        trailing: Vec::new(),
        raw_name: None,
    });
    let mut recorder = Recorder::new();
    recorder
        .start(&library, Selection::Action { set: 0, action: 0 })
        .expect("start");
    let stroke = pictura_script::ToolStroke::new(
        "brush",
        CommandParams::new().with(
            "blob",
            "x".repeat(pictura_script::recorder::MAX_TOOL_PAYLOAD_BYTES),
        ),
        vec![],
    );
    assert!(matches!(
        recorder.record_tool_stroke(&mut library, true, stroke),
        Err(ActionError::ToolStrokeTooLarge { .. })
    ));
    assert!(recorder.is_recording());

    let sample = pictura_script::ToolSample {
        x: 0.0,
        y: 0.0,
        pressure: 0.0,
        time: 0.0,
    };
    let too_many = pictura_script::ToolStroke::new(
        "brush",
        CommandParams::new(),
        vec![sample; pictura_script::recorder::MAX_TOOL_SAMPLES + 1],
    );
    assert!(matches!(
        recorder.record_tool_stroke(&mut library, true, too_many),
        Err(ActionError::ToolStrokeTooLarge { .. })
    ));
}
