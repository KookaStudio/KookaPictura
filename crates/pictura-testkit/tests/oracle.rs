use pictura_testkit::{compare, oracle};

#[test]
#[should_panic(expected = "oracle mismatch")]
fn present_oracle_with_wrong_answer_fails() {
    let output = oracle::Command::new("python3")
        .args(["-c", "print('wrong')"])
        .output()
        .expect("run the present oracle");
    assert!(output.status.success(), "{output:?}");
    let diff = compare(b"right\n", &output.stdout, 0).expect("equal-length buffers");
    assert!(diff.is_empty(), "oracle mismatch: {diff:?}");
}
