use genja_core_derive::{DerefMacro, DerefMutMacro};

trait DerefTarget {
    type Target;
}

#[derive(DerefMacro, DerefMutMacro)]
struct Values(Vec<String>);

impl DerefTarget for Values {
    type Target = Vec<String>;
}

fn main() {
    let mut values = Values(Vec::new());
    values.push("one".to_string());

    assert_eq!(values.as_slice(), ["one".to_string()]);
}
