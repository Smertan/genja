use genja_core::inventory::{
    BaseBuilderHost, ConnectionManager, Data, Host, Hosts, Inventory, TransformFunction,
    TransformFunctionOptions,
};
use serde_json::json;
// use std::sync::Arc;

pub fn inventory_setup() -> Result<Inventory, Box<dyn std::error::Error>> {
    let transform_options: TransformFunctionOptions = serde_json::from_value(json!({
        "obfuscated_ip_map": {
            "10-0-0-1": "10.0.0.1",
            "10-0-0-2": "10.0.0.2"
        }
    }))
    .expect("transform options should deserialize");

    let transform_function = TransformFunction::new(
        |inventory: &mut Inventory, options: Option<&TransformFunctionOptions>| {
            let mapping = options
                .and_then(|opts| opts.get("obfuscated_ip_map"))
                .and_then(|value| value.as_object());
            let Some(mapping) = mapping else {
                return;
            };

            for host in inventory.hosts_mut().values_mut() {
                if let Some(hostname) = host.hostname.as_mut() {
                    if let Some(mapped) = mapping.get(hostname).and_then(|value| value.as_str()) {
                        *hostname = mapped.to_string();
                    }
                }

                if let Some(data) = host.data.as_mut() {
                    if let Some(object) = data.as_object_mut() {
                        if let Some(mgmt_ip) = object.get_mut("mgmt_ip") {
                            if let Some(ip) = mgmt_ip.as_str() {
                                if let Some(mapped) =
                                    mapping.get(ip).and_then(|value| value.as_str())
                                {
                                    *mgmt_ip = serde_json::Value::String(mapped.to_string());
                                }
                            }
                        }
                    }
                }
            }
        },
    );

    let mut hosts = Hosts::new();
    let host1 = Host::builder("router1.lab")
        .hostname("10-0-0-1")
        .data(Data::new(json!({ "mgmt_ip": "10-0-0-1" })))
        .build();
    let host2 = Host::builder("switch1.lab")
        .hostname("10-0-0-2")
        .data(Data::new(json!({ "mgmt_ip": "10-0-0-2" })))
        .build();

    hosts.add_host(host1);
    hosts.add_host(host2);

    let inventory = Inventory::builder()
        .hosts(hosts)
        .transform_function(transform_function)
        .transform_function_options(transform_options)
        .connections(ConnectionManager::default())
        .build();
    Ok(inventory)
}
