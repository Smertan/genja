use genja_core_derive::DerefMutMacro;

#[derive(DerefMutMacro)]
enum Values {
    Items(Vec<String>),
}

fn main() {}
