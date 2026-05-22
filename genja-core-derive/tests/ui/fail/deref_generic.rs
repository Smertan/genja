use genja_core_derive::DerefMacro;

#[derive(DerefMacro)]
struct GenericValues<T>(Vec<T>);

fn main() {}
