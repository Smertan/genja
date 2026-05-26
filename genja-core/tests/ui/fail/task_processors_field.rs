use genja_core_derive::Task;

#[derive(Task)]
struct ProcessorsField {
    name: &'static str,
    processors: Vec<String>,
}

fn main() {}
