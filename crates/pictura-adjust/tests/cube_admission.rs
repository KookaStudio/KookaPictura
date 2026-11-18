use pictura_adjust::parse_cube;

fn cube(size: usize, count: usize) -> Vec<u8> {
    let mut bytes = format!("LUT_3D_SIZE {size}\n").into_bytes();
    for _ in 0..count {
        bytes.extend_from_slice(b"0.25 0.5 0.75\n");
    }
    bytes
}

#[test]
fn later_cube_size_declarations_keep_their_existing_meaning() {
    let mut bytes = cube(2, 27);
    bytes.extend_from_slice(b"LUT_3D_SIZE 3\n");
    let parsed = parse_cube(&bytes).expect("the final declaration determines the exact count");
    assert_eq!(parsed.size, 3);
    assert_eq!(parsed.points, vec![[0.25, 0.5, 0.75]; 27]);
}

#[test]
fn the_supported_cube_maximum_remains_valid() {
    let count = 64 * 64 * 64;
    let parsed = parse_cube(&cube(64, count)).expect("the supported maximum remains readable");
    assert_eq!(parsed.size, 64);
    assert_eq!(parsed.points.len(), count);
    assert_eq!(parsed.points[0], [0.25, 0.5, 0.75]);
    assert_eq!(parsed.points[count - 1], [0.25, 0.5, 0.75]);
}

#[test]
fn points_beyond_the_supported_maximum_keep_the_no_lookup_result() {
    let bytes = cube(64, 64 * 64 * 64 + 1);
    let original = bytes.clone();
    assert_eq!(parse_cube(&bytes), None);
    assert_eq!(bytes, original);
}
