use genja_core_derive::Task;

#[derive(Task)]
struct WrongProcessorNamesType {
    name: &'static str,
    processor_names: Vec<&'static str>,
}

fn main() {}
