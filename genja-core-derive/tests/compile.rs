#[test]
fn task_derive_passes_supported_inputs() {
    let test_cases = trybuild::TestCases::new();
    test_cases.pass("tests/ui/pass/*.rs");
}

#[test]
fn task_derive_rejects_unsupported_inputs() {
    let test_cases = trybuild::TestCases::new();
    test_cases.compile_fail("tests/ui/fail/*.rs");
}
