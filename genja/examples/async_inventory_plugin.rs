use genja::genja_core::async_trait;
use genja::genja_core::inventory::{BaseBuilderHost, Data, Host, Hosts, Inventory};
use genja::genja_core::task::{HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess};
use genja::genja_core::{InventoryLoadError, Settings};
use genja::{Genja, genja_task};
use genja_plugin_manager::PluginManager;
use genja_plugin_manager::plugin_types::{AsyncPluginInventory, Plugin, Plugins};
use serde_json::json;

#[derive(Debug)]
struct ControllerInventoryPlugin;

#[derive(Debug)]
struct ControllerDevice {
    host_id: &'static str,
    hostname: &'static str,
    platform: &'static str,
    role: &'static str,
}

impl Plugin for ControllerInventoryPlugin {
    fn name(&self) -> String {
        "controller_inventory".to_string()
    }
}

#[async_trait]
impl AsyncPluginInventory for ControllerInventoryPlugin {
    async fn load_async(
        &self,
        _settings: &Settings,
        _plugins: &PluginManager,
    ) -> Result<Inventory, InventoryLoadError> {
        let devices = fetch_controller_inventory().await?;
        let mut hosts = Hosts::new();

        for device in devices {
            hosts.add_host(
                device.host_id,
                Host::builder()
                    .hostname(device.hostname)
                    .platform(device.platform)
                    .data(Data::new(json!({ "role": device.role })))
                    .build(),
            );
        }

        Ok(Inventory::builder().hosts(hosts).build())
    }
}

async fn fetch_controller_inventory() -> Result<Vec<ControllerDevice>, InventoryLoadError> {
    // In a real plugin this would be an async HTTP/database/service-discovery call.
    tokio::task::yield_now().await;

    Ok(vec![
        ControllerDevice {
            host_id: "router1",
            hostname: "10.0.0.1",
            platform: "ios",
            role: "core",
        },
        ControllerDevice {
            host_id: "router2",
            hostname: "10.0.0.2",
            platform: "nxos",
            role: "edge",
        },
    ])
}

struct CollectFacts;

#[genja_task(name = "collect_facts")]
impl CollectFacts {
    async fn start_async(
        &self,
        host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new().with_result(
            json!({
                "hostname": host.hostname(),
                "platform": host.platform(),
                "role": host.data().and_then(|data| data.get("role")),
                "facts_collected": true
            }),
        )))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::from_file("genja/examples/settings.yaml")?;

    let mut plugins = PluginManager::new();
    plugins.register_plugin(Plugins::AsyncInventory(Box::new(ControllerInventoryPlugin)));

    let inventory = plugins
        .get_async_inventory_plugin("controller_inventory")
        .ok_or("missing async inventory plugin")?
        .load_async(&settings, &plugins)
        .await?;

    let genja = Genja::builder(inventory)
        .with_settings(settings)
        .with_plugin_manager(plugins)
        .build()?;

    let results = genja.run_task_async(CollectFacts, 1).await?;

    let output = results.to_pretty_json_string()?;
    println!("{output}");

    Ok(())
}
