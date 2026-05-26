use genja_core_derive::Task;

#[derive(Task)]
#[task(subtask)]
struct StructSubtaskAttributeTask {
    name: &'static str,
}

fn main() {}
