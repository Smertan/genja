use genja_core_derive::Task;

#[derive(Task)]
struct WrongSubtaskType {
    name: &'static str,
    #[task(subtask)]
    child: String,
}

fn main() {}
