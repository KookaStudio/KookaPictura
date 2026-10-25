use super::*;

#[test]
fn tool_catalogue_rows_unique_and_grouped() {
    let rows = tool_catalogue();
    assert_eq!(rows.len(), 71);
    for (i, row) in rows.iter().enumerate() {
        assert!(
            rows[i + 1..].iter().all(|other| other.name != row.name),
            "{} appears twice",
            row.name
        );
        assert_eq!(tool_spec(row.name), Some(row));
        assert!(row.group > 0, "{} has a flyout group", row.name);
    }
    for group in rows.iter().map(|row| row.group) {
        let letters: Vec<char> = rows
            .iter()
            .filter(|row| row.group == group)
            .filter_map(|row| row.shortcut)
            .collect();
        assert!(
            letters.windows(2).all(|pair| pair[0] == pair[1]),
            "group {group} shares one letter: {letters:?}"
        );
    }
    let without: Vec<&str> = rows
        .iter()
        .filter(|row| row.shortcut.is_none())
        .map(|row| row.name)
        .collect();
    assert_eq!(
        without,
        [
            "blur",
            "sharpen",
            "smudge",
            "addanchorpoint",
            "deleteanchorpoint",
            "convertpoint"
        ]
    );
    let free_transform = &tool_spec("move").unwrap().modifier_hints[2];
    assert_eq!(free_transform.command_id, "edit.freeTransform");
}

#[test]
fn tool_catalogue_implemented_set() {
    let implemented: Vec<&str> = tool_catalogue()
        .iter()
        .filter(|row| row.implemented)
        .map(|row| row.name)
        .collect();
    assert_eq!(implemented.len(), 61);
    assert_eq!(implemented.first(), Some(&"move"));
    assert_eq!(implemented.last(), Some(&"zoom"));
    for name in ["objectrotate", "camerazoom"] {
        assert!(!tool_spec(name).unwrap().implemented, "{name} is a 3D tool");
    }
}
