use genja_core_derive::Task;

#[derive(Task)]
struct GenericTask<T> {
    name: &'static str,
    value: T,
}

fn main() {}
