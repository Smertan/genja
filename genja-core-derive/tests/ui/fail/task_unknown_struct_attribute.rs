use genja_core_derive::Task;

#[derive(Task)]
#[task(foo)]
struct UnknownStructAttributeTask {
    name: &'static str,
}

fn main() {}
