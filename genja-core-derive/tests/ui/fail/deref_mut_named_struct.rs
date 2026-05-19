use genja_core_derive::DerefMutMacro;

#[derive(DerefMutMacro)]
struct Values {
    inner: Vec<String>,
}

fn main() {}
