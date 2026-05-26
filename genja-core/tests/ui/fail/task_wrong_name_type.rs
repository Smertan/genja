use genja_core_derive::Task;

#[derive(Task)]
struct WrongNameType {
    name: usize,
}

fn main() {}
