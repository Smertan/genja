use genja_core_derive::DerefMutMacro;

#[derive(DerefMutMacro)]
struct GenericValues<T>(Vec<T>);

fn main() {}
