pub mod task_a;
use plugin_manager::plugin_types::Plugins;

#[unsafe(no_mangle)]
pub fn create_plugins() -> Vec<Plugins> {
    let plugins = vec![Plugins::Base(Box::new(task_a::TaskA))];
    plugins
}
