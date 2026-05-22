use genja_core::task::TaskInfo;
use genja_core_derive::Task;

#[derive(Task)]
#[task(processors = ["audit", "metrics"])]
struct StaticTask {
    name: &'static str,
}

fn main() {
    let task = StaticTask { name: "deploy" };

    assert_eq!(task.name(), "deploy");
    assert_eq!(task.processor_names(), vec!["audit", "metrics"]);
}
