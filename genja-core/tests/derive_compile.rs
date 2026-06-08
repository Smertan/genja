#[test]
fn genja_task_passes_supported_inputs() {
    let test_cases = trybuild::TestCases::new();
    test_cases.pass("tests/ui_genja_task/pass/*.rs");
}

#[test]
fn genja_task_rejects_unsupported_inputs() {
    let test_cases = trybuild::TestCases::new();
    test_cases.compile_fail("tests/ui_genja_task/fail/*.rs");
}
