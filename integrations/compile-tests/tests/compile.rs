#[test]
pub fn reject() {
    trybuild::TestCases::new().compile_fail("tests/fail/*.rs");
}

#[test]
pub fn accept() {
    trybuild::TestCases::new().pass("tests/pass/*.rs");
}
