pub mod task_a;
use genja_plugin_manager::plugin_types::Plugins;

#[unsafe(no_mangle)]
pub fn create_plugins() -> Vec<Plugins> {
    let plugins = vec![Plugins::Connection(
        Box::new(task_a::TaskA::new_prototype()),
    )];
    plugins
}
