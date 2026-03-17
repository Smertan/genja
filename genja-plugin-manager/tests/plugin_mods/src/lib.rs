pub mod plugin_a;
pub mod plugin_b;
use plugin_manager::plugin_types::Plugins;

#[unsafe(no_mangle)]
pub fn create_plugins() -> Vec<Plugins> {
    let plugins = vec![
        Plugins::Connection(Box::new(plugin_a::PluginA::new_prototype())),
        Plugins::Connection(Box::new(plugin_b::PluginB::new_prototype())),
    ];
    plugins
}
