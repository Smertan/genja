use genja_core_derive::Task;

#[derive(Task)]
struct WrongOptionsType {
    name: &'static str,
    options: serde_json::Value,
}

fn main() {}
