use genja_core_derive::Task;

#[derive(Task)]
struct FieldProcessorsAttributeTask {
    name: &'static str,
    #[task(processors = ["audit"])]
    label: String,
}

fn main() {}
