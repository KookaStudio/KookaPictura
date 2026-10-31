//! The tool catalogue. Arguments pass through: `ControlServer` validates them, so
//! a second copy of those rules here could only drift.

use serde_json::{json, Value};

struct Tool {
    name: &'static str,
    method: &'static str,
    description: &'static str,
    properties: fn() -> Value,
    required: &'static [&'static str],
}

fn none() -> Value {
    json!({})
}

fn document() -> Value {
    json!({
        "op": {"type": "string", "enum": ["list", "open", "new", "save", "save_as", "revert", "close", "activate"]},
        "path": {"type": "string", "description": "Absolute path, for open, save and save_as"},
        "index": {"type": "integer"},
        "width": {"type": "integer"},
        "height": {"type": "integer"},
        "name": {"type": "string"},
        "mode": {"type": "string", "enum": ["rgb", "grayscale"]},
        "background": {"type": "string", "enum": ["white", "transparent"]},
    })
}

fn pixel() -> Value {
    json!({"x": {"type": "integer"}, "y": {"type": "integer"}})
}

fn screenshot() -> Value {
    json!({
        "scope": {"type": "string", "enum": ["window", "canvas", "document"]},
        "max_dim": {"type": "integer", "description": "Longest edge; default 1280"},
    })
}

fn thumbnail() -> Value {
    json!({"index": {"type": "integer"}, "size": {"type": "integer"}})
}

fn implemented_only() -> Value {
    json!({"implemented_only": {"type": "boolean"}})
}

fn filter() -> Value {
    json!({
        "kind": {"type": "string", "description": "From pictura_list_filters"},
        "params": {"type": "array", "items": {"type": "number"}, "description": "Slot values; omit for defaults"},
        "doc": {"type": "string", "description": "Document id; refused if it is no longer active"},
    })
}

fn adjust() -> Value {
    json!({
        "kind": {"type": "string", "description": "From pictura_list_adjustments"},
        "values": {"type": "object", "additionalProperties": {"type": "number"}},
        "doc": {"type": "string"},
    })
}

fn adjustment_layer() -> Value {
    json!({"kind": {"type": "string"}})
}

fn run_command() -> Value {
    json!({"id": {"type": "string"}, "params": {"type": "object"}, "doc": {"type": "string"}})
}

fn dispatch_command() -> Value {
    json!({"id": {"type": "string"}, "doc": {"type": "string"}})
}

fn selection() -> Value {
    json!({
        "op": {"type": "string", "enum": ["all", "deselect", "rect", "ellipse", "lasso", "quick", "wand"]},
        "x": {"type": "integer"}, "y": {"type": "integer"},
        "w": {"type": "integer"}, "h": {"type": "integer"},
        "points": {"type": "array", "items": {"type": "array", "items": {"type": "integer"}}},
        "mode": {"type": "string", "enum": ["new", "add", "subtract", "intersect"]},
        "feather": {"type": "number"},
        "tolerance": {"type": "integer"},
        "contiguous": {"type": "boolean"},
    })
}

fn layer_op() -> Value {
    json!({
        "op": {"type": "string"},
        "index": {"type": "integer"},
        "name": {"type": "string"},
        "doc": {"type": "string"},
    })
}

fn layer_path() -> Value {
    json!({"path": {"type": "string", "description": "From pictura_layer_tree"}})
}

fn tool() -> Value {
    json!({"tool": {"type": "string"}})
}

fn tool_options() -> Value {
    json!({"tool": {"type": "string"}, "options": {"type": "object"}})
}

fn pointer() -> Value {
    json!({
        "op": {"type": "string", "enum": ["click", "dblclick", "move", "drag", "scroll", "down", "up"]},
        "space": {"type": "string", "enum": ["image"]},
        "x": {"type": "number"}, "y": {"type": "number"},
        "x2": {"type": "number"}, "y2": {"type": "number"},
        "button": {"type": "string"},
        "steps": {"type": "integer"},
    })
}

fn edit() -> Value {
    json!({
        "op": {"type": "string", "enum": ["undo", "redo"]},
        "expected_index": {"type": "integer", "description": "Refused if history has moved"},
    })
}

fn history() -> Value {
    json!({
        "op": {"type": "string", "enum": ["list", "jump", "snapshot", "restore"]},
        "index": {"type": "integer"},
        "label": {"type": "string"},
    })
}

fn export_png() -> Value {
    json!({"path": {"type": "string", "description": "Absolute path"}, "doc": {"type": "string"}})
}

// `key`, `set_unsaved_policy` and `set_gpu_compute` are left out: restricted
// sessions refuse them, and an agent has no use for a tool that always fails.
const TOOLS: &[Tool] = &[
    Tool { name: "pictura_status", method: "status", description: "Open documents, their ids, the active tool and the app state.", properties: none, required: &[] },
    Tool { name: "pictura_document", method: "document", description: "List, open, create, save, revert, close or activate documents.", properties: document, required: &["op"] },
    Tool { name: "pictura_get_pixel", method: "get_pixel", description: "Read one composited pixel.", properties: pixel, required: &["x", "y"] },
    Tool { name: "pictura_screenshot", method: "screenshot", description: "See the window, canvas or document as an image.", properties: screenshot, required: &[] },
    Tool { name: "pictura_list_layers", method: "list_layers", description: "Top-level layers with their blend and opacity, and the active layer.", properties: none, required: &[] },
    Tool { name: "pictura_layer_tree", method: "layer_tree", description: "Every layer, including those in groups, with its path.", properties: none, required: &[] },
    Tool { name: "pictura_layer_thumbnail", method: "layer_thumbnail", description: "A thumbnail of one top-level layer.", properties: thumbnail, required: &["index"] },
    Tool { name: "pictura_ui_tree", method: "ui_tree", description: "The widget tree of the window.", properties: none, required: &[] },
    Tool { name: "pictura_metrics", method: "metrics", description: "Render timing counters.", properties: none, required: &[] },
    Tool { name: "pictura_list_commands", method: "list_commands", description: "Menu commands with their ids and state.", properties: implemented_only, required: &[] },
    Tool { name: "pictura_list_filters", method: "list_filters", description: "Filter kinds, their parameters and ranges.", properties: none, required: &[] },
    Tool { name: "pictura_list_adjustments", method: "list_adjustments", description: "Adjustment kinds and their parameters.", properties: none, required: &[] },
    Tool { name: "pictura_filter", method: "filter", description: "Apply a filter to the active layer, within any selection.", properties: filter, required: &["kind"] },
    Tool { name: "pictura_adjust", method: "adjust_apply", description: "Apply an adjustment directly to the active layer.", properties: adjust, required: &["kind"] },
    Tool { name: "pictura_add_adjustment_layer", method: "adjustment", description: "Add an adjustment layer.", properties: adjustment_layer, required: &["kind"] },
    Tool { name: "pictura_run_command", method: "run_command", description: "Run a command with parameters: image.image.size, image.canvas.size, image.rotate.arbitrary, edit.fill, mode changes, layer.duplicate.layer, layer.flatten.image.", properties: run_command, required: &["id"] },
    Tool { name: "pictura_dispatch_command", method: "dispatch_command", description: "Run a parameterless menu command. Those that need a dialog are refused.", properties: dispatch_command, required: &["id"] },
    Tool { name: "pictura_selection", method: "selection", description: "Make or clear a selection.", properties: selection, required: &["op"] },
    Tool { name: "pictura_layer_op", method: "layer_op", description: "Add, delete, rename, reorder or restyle layers.", properties: layer_op, required: &["op"] },
    Tool { name: "pictura_set_active_layer", method: "set_active_layer", description: "Make the layer at a path active.", properties: layer_path, required: &["path"] },
    Tool { name: "pictura_set_tool", method: "set_tool", description: "Choose a tool.", properties: tool, required: &["tool"] },
    Tool { name: "pictura_tool_options", method: "tool_options", description: "Read or change a tool's options.", properties: tool_options, required: &[] },
    Tool { name: "pictura_pointer", method: "pointer", description: "Drive the active tool on the canvas, in image coordinates.", properties: pointer, required: &["op", "space"] },
    Tool { name: "pictura_commit_crop", method: "commit_crop", description: "Apply a crop you started with pictura_pointer.", properties: none, required: &[] },
    Tool { name: "pictura_edit", method: "edit", description: "Undo or redo.", properties: edit, required: &["op"] },
    Tool { name: "pictura_history", method: "history", description: "List history, jump to a state, or make and restore snapshots.", properties: history, required: &["op"] },
    Tool { name: "pictura_export_png", method: "export_png", description: "Write a flattened PNG. The document is not changed.", properties: export_png, required: &["path"] },
];

pub fn listing() -> Value {
    TOOLS
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": {
                    "type": "object",
                    "properties": (tool.properties)(),
                    "required": tool.required,
                },
            })
        })
        .collect()
}

pub fn control_method(name: &str) -> Option<&'static str> {
    TOOLS
        .iter()
        .find(|tool| tool.name == name)
        .map(|tool| tool.method)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_are_listed_safely() {
        let listed = listing();
        let tools = listed.as_array().unwrap();
        assert_eq!(tools.len(), TOOLS.len());
        for tool in tools {
            assert!(tool["name"].as_str().unwrap().starts_with("pictura_"));
            assert_eq!(tool["inputSchema"]["type"], "object");
            assert!(!tool["description"].as_str().unwrap().is_empty());
        }
        for tool in TOOLS {
            for required in tool.required {
                assert!(
                    (tool.properties)().get(required).is_some(),
                    "{}: {required}",
                    tool.name
                );
            }
            assert!(!["key", "set_unsaved_policy", "set_gpu_compute"].contains(&tool.method));
        }
    }
}
