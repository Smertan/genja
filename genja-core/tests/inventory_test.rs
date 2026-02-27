use genja_core::inventory::{
    BaseBuilderHost, ConnectionKey, ConnectionManager, ConnectionOptions, Data, Defaults, Extras,
    Group, Groups, Host, Hosts, Inventory, ParentGroups, TransformFunctionOptions,
};
// use genja_core::CustomTreeMap;
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
    ConnectionOptions::builder()
        .hostname(hostname)
        .port(port)
        .username(username)
        .password(password)
        .platform(platform)
        .build()
}

#[test]
fn inventory_can_model_mock_network_devices() {
    let defaults: Defaults = serde_json::from_value(json!({
        "transport": "ssh",
        "connection_timeout": 30,
        "global_retries": 2
    }))
    .expect("defaults should deserialize");

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

    let router = Host::builder()
        .hostname("router1.lab")
        .platform("cisco_ios")
        .groups(router_groups)
        .data(router_data)
        .connection_options("netconf", router_connection)
        .build();
    hosts.add_host("router1.lab", router);

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

    let switch = Host::builder()
        .hostname("switch1.lab")
        .platform("nxos")
        .groups(switch_groups)
        .data(switch_data)
        .connection_options("netconf", switch_connection)
        .build();
    hosts.add_host("switch1.lab", switch);

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
    assert_eq!(router.hostname(), Some("router1.lab"));
    assert_eq!(router.groups(), Some(&router_groups_snapshot));
    assert_eq!(router.data(), Some(&router_data_snapshot));
    assert_eq!(
        router
            .connection_options()
            .and_then(|options| options.get("netconf")),
        Some(&router_connection_snapshot)
    );

    let switch = inventory
        .hosts()
        .get("switch1.lab")
        .expect("switch host should exist");
    assert_eq!(switch.hostname(), Some("switch1.lab"));
    assert_eq!(switch.groups(), Some(&switch_groups_snapshot));
    assert_eq!(switch.data(), Some(&switch_data_snapshot));
    assert_eq!(
        switch
            .connection_options()
            .and_then(|options| options.get("netconf")),
        Some(&switch_connection_snapshot)
    );

    assert_eq!(inventory.defaults(), Some(defaults.clone()));
    assert_eq!(
        inventory.transform_function_options(),
        Some(&transform_options)
    );

    let resolved = inventory
        .resolve_connection_params("router1.lab", "netconf")
        .expect("resolved params should exist");
    assert_eq!(resolved.hostname, "192.0.2.1");
    assert_eq!(resolved.port, Some(22));
}

#[test]
fn inventory_transform_translates_obfuscated_ips() {
    let inventory = common::inventory_setup().expect("inventory setup failed");

    let router = inventory
        .hosts()
        .get("router1.lab")
        .expect("router should exist");
    assert_eq!(router.hostname(), Some("10.0.0.1"));
    assert_eq!(
        router
            .data()
            .and_then(|data| data.get("mgmt_ip"))
            .and_then(|value| value.as_str()),
        Some("10.0.0.1")
    );

    let switch = inventory
        .hosts()
        .get("switch1.lab")
        .expect("switch should exist");
    assert_eq!(switch.hostname(), Some("10.0.0.2"));
    assert_eq!(
        switch
            .data()
            .and_then(|data| data.get("mgmt_ip"))
            .and_then(|value| value.as_str()),
        Some("10.0.0.2")
    );
}

#[test]
fn connection_options_precedence_defaults_groups_hosts() {
    let defaults: Defaults = serde_json::from_value(json!({
        "hostname": "default-host",
        "port": 1001,
        "connection_options": {
            "netconf": {
                "hostname": "default-netconf",
                "port": 2001,
                "username": "default-user",
                "extras": { "tier": "defaults" }
            }
        }
    }))
    .expect("defaults should deserialize");

    let group_netconf = ConnectionOptions::builder()
        .hostname("group-netconf")
        .username("group-user")
        .extras(Extras::new(json!({ "tier": "group" })))
        .build();

    let group = Group::builder()
        .hostname("group-host")
        .port(1002)
        .connection_options("netconf", group_netconf)
        .build();

    let mut groups = Groups::new();
    groups.add_group("core", group);

    let host_netconf = ConnectionOptions::builder()
        .hostname("host-netconf")
        .port(2003)
        .extras(Extras::new(json!({ "tier": "host" })))
        .build();

    let host = Host::builder()
        .hostname("host-host")
        .port(1003)
        .groups({
            let mut pg = ParentGroups::new();
            pg.push("core".to_string());
            pg
        })
        .connection_options("netconf", host_netconf)
        .build();

    let mut hosts = Hosts::new();
    hosts.add_host("router1.lab", host);

    let inventory = Inventory::builder()
        .hosts(hosts)
        .groups(groups)
        .defaults(defaults)
        .connections(ConnectionManager::default())
        .build();

    let resolved = inventory
        .resolve_connection_params("router1.lab", "netconf")
        .expect("resolved params should exist");

    assert_eq!(resolved.hostname, "host-netconf");
    assert_eq!(resolved.port, Some(2003));
    assert_eq!(resolved.username.as_deref(), Some("group-user"));
    assert_eq!(resolved.password, None);
    assert_eq!(resolved.platform, None);
    assert_eq!(
        resolved.extras,
        Some(Extras::new(json!({ "tier": "host" })))
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
