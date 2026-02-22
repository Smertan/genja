use genja_core::inventory::{
    BaseBuilderHost, ConnectionKey, ConnectionManager, ConnectionOptions, Data, Defaults, Host,
    Hosts, Inventory, ParentGroups, TransformFunctionOptions,
};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
mod common;

fn build_connection_options(
    hostname: &str,
    port: u16,
    username: &str,
    password: &str,
    platform: &str,
) -> ConnectionOptions {
    let mut options = ConnectionOptions::new();
    options.hostname = Some(hostname.to_string());
    options.port = Some(port);
    options.username = Some(username.to_string());
    options.password = Some(password.to_string());
    options.platform = Some(platform.to_string());
    options
}

#[test]
fn inventory_can_model_mock_network_devices() {
    let defaults: Defaults = serde_json::from_value(json!({
        "transport": "ssh",
        "connection_timeout": 30,
        "global_retries": 2
    }))
    .expect("defaults should deserialize");
    let defaults_arc = Arc::new(defaults.clone());

    let transform_options: TransformFunctionOptions =
        serde_json::from_value(json!({ "strip_domain": true, "sanitize_credentials": true }))
            .expect("transform options should deserialize");

    let mut hosts = Hosts::new();

    // Router mock device
    let mut router_groups = ParentGroups::new();
    router_groups.push("core".into());
    router_groups.push("routers".into());
    let router_groups_snapshot = router_groups.clone();

    let router_data = Data::new(json!({
        "role": "core_router",
        "mgmt_ip": "192.0.2.1"
    }));
    let router_data_snapshot = router_data.clone();

    let router_connection =
        build_connection_options("192.0.2.1", 22, "automation", "router_pass", "cisco_ios");
    let router_connection_snapshot = router_connection.clone();

    let router = Host::builder("router1.lab")
        .hostname("router1.lab")
        .platform("cisco_ios")
        .groups(router_groups)
        .data(router_data)
        .connection_options("netconf".into(), router_connection)
        .defaults(&defaults_arc)
        .build();
    hosts.add_host(router);

    // Switch mock device
    let mut switch_groups = ParentGroups::new();
    switch_groups.push("access".into());
    switch_groups.push("switches".into());
    let switch_groups_snapshot = switch_groups.clone();

    let switch_data = Data::new(json!({
        "role": "access_switch",
        "mgmt_ip": "192.0.2.10"
    }));
    let switch_data_snapshot = switch_data.clone();

    let switch_connection =
        build_connection_options("192.0.2.10", 2222, "netops", "switch_pass", "nxos");
    let switch_connection_snapshot = switch_connection.clone();

    let switch = Host::builder("switch1.lab")
        .hostname("switch1.lab")
        .platform("nxos")
        .groups(switch_groups)
        .data(switch_data)
        .connection_options("netconf".into(), switch_connection)
        .defaults(&defaults_arc)
        .build();
    hosts.add_host(switch);

    let inventory = Inventory::builder()
        .hosts(hosts)
        .defaults(defaults.clone())
        .transform_function_options(transform_options.clone())
        .connections(ConnectionManager::default())
        .build();

    assert_eq!(inventory.hosts().len(), 2);

    let router = inventory
        .hosts()
        .get("router1.lab")
        .expect("router host should exist");
    assert_eq!(router.hostname.as_deref(), Some("router1.lab"));
    assert_eq!(router.groups.as_ref(), Some(&router_groups_snapshot));
    assert_eq!(router.data.as_ref(), Some(&router_data_snapshot));
    assert_eq!(
        router.defaults.as_ref(),
        Some(&defaults_arc),
        "router should inherit defaults"
    );
    assert_eq!(
        router
            .connection_options
            .as_ref()
            .and_then(|options| options.get("netconf")),
        Some(&router_connection_snapshot)
    );

    let switch = inventory
        .hosts()
        .get("switch1.lab")
        .expect("switch host should exist");
    assert_eq!(switch.hostname.as_deref(), Some("switch1.lab"));
    assert_eq!(switch.groups.as_ref(), Some(&switch_groups_snapshot));
    assert_eq!(switch.data.as_ref(), Some(&switch_data_snapshot));
    assert_eq!(
        switch
            .connection_options
            .as_ref()
            .and_then(|options| options.get("netconf")),
        Some(&switch_connection_snapshot)
    );

    assert_eq!(inventory.defaults(), Some(&defaults));
    assert_eq!(
        inventory.transform_function_options(),
        Some(&transform_options)
    );
}

#[test]
fn inventory_transform_translates_obfuscated_ips() {
    let mut inventory = common::inventory_setup().expect("inventory setup failed");
    inventory.apply_transform();

    let router = inventory
        .hosts()
        .get("router1.lab")
        .expect("router should exist");
    assert_eq!(router.hostname.as_deref(), Some("10.0.0.1"));
    assert_eq!(
        router
            .data
            .as_ref()
            .and_then(|data| data.get("mgmt_ip"))
            .and_then(|value| value.as_str()),
        Some("10.0.0.1")
    );

    let switch = inventory
        .hosts()
        .get("switch1.lab")
        .expect("switch should exist");
    assert_eq!(switch.hostname.as_deref(), Some("10.0.0.2"));
    assert_eq!(
        switch
            .data
            .as_ref()
            .and_then(|data| data.get("mgmt_ip"))
            .and_then(|value| value.as_str()),
        Some("10.0.0.2")
    );
}

#[test]
fn connection_manager_creates_connections_lazily() {
    #[derive(Debug)]
    struct TestConnection;

    impl genja_core::inventory::Connection for TestConnection {
        fn is_alive(&self) -> bool {
            true
        }

        fn open(
            &mut self,
            _params: &genja_core::inventory::ResolvedConnectionParams,
        ) -> Result<(), String> {
            Ok(())
        }

        fn close(&mut self) -> ConnectionKey {
            ConnectionKey::new("router1.lab", "ssh2")
        }
    }

    let manager = ConnectionManager::default();
    let key = ConnectionKey::new("router1.lab", "ssh2");
    let created = AtomicUsize::new(0);

    let first = manager.get_or_create(key.clone(), || {
        created.fetch_add(1, Ordering::SeqCst);
        TestConnection
    });
    let second = manager.get_or_create(key, || {
        created.fetch_add(1, Ordering::SeqCst);
        TestConnection
    });

    assert_eq!(created.load(Ordering::SeqCst), 1);
    assert!(Arc::ptr_eq(&first, &second));
}
