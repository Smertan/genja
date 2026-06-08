pub mod connection_a;
use genja_plugin_manager::plugin_types::Plugins;

#[unsafe(no_mangle)]
pub fn create_plugins() -> Vec<Plugins> {
    let plugins = vec![Plugins::Connection(Box::new(
        connection_a::ConnectionA::new_prototype(),
    ))];
    plugins
}
