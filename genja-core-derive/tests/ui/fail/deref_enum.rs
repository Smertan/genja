use genja_core_derive::DerefMacro;

#[derive(DerefMacro)]
enum Values {
    Items(Vec<String>),
}

fn main() {}
