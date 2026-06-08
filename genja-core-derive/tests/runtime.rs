use genja_core_derive::{DerefMacro, DerefMutMacro};

trait DerefTarget {
    type Target;
}

#[derive(DerefMacro, DerefMutMacro)]
struct Values(Vec<String>);

impl DerefTarget for Values {
    type Target = Vec<String>;
}

#[test]
fn deref_macros_read_and_mutate_wrapped_value() {
    let mut values = Values(vec!["one".to_string()]);

    assert_eq!(values.as_slice(), ["one".to_string()]);

    values.push("two".to_string());

    assert_eq!(values.as_slice(), ["one".to_string(), "two".to_string()]);
}
