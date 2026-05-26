use genja_core_derive::Task;

#[derive(Task)]
#[task(processors = ["audit"])]
struct DuplicateProcessorSources {
    name: &'static str,
    processor_names: Vec<String>,
}

fn main() {}
