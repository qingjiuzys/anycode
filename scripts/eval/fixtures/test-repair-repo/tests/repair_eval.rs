use eval_test_repair::add;

#[test]
fn adds_one_and_two() {
    // Wrong expectation (written as if add subtracted).
    assert_eq!(add(1, 2), 4);
}
