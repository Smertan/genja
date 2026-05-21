use genja_core_derive::Task;

#[derive(Task)]
struct UnknownFieldAttributeTask {
    name: &'static str,
    #[task(not_subtask)]
    label: String,
}

fn main() {}
