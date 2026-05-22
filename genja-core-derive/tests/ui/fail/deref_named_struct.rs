use genja_core_derive::DerefMacro;

#[derive(DerefMacro)]
struct Values {
    inner: Vec<String>,
}

fn main() {}
