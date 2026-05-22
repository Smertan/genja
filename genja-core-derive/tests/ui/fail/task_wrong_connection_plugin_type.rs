use genja_core_derive::Task;

#[derive(Task)]
struct WrongConnectionPluginType {
    name: &'static str,
    connection_plugin_name: usize,
}

fn main() {}
