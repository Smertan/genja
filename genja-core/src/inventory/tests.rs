mod tests {
    use super::*;
    use crate::CustomTreeMap;
    use async_trait::async_trait;
    use std::future::Future;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::runtime::Builder;
    use tokio::sync::Mutex;

    fn run_async<F: Future>(future: F) -> F::Output {
        Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should build")
            .block_on(future)
    }

    fn create_dummy_hosts() -> Result<Hosts, std::io::Error> {
        let mut hosts = Hosts(CustomTreeMap::new());
        // hosts.insert("hosts".to_string(), CustomTreeMap::new());
        for i in 1..=10 {
            let name = format!("host{i}.example.com");
            let mut groups = ParentGroups::new();
            groups.push("cisco".to_string());
            let host = Host::builder()
                .hostname(&name)
                .port(2200 + i as u16)
                .username(format!("user{i}"))
                .password(format!("password{i}"))
                .platform(if i % 2 == 0 { "linux" } else { "windows" })
                .data(Data(serde_json::json!(vec![format!(
                    "data for host {}",
                    i
                )])))
                .groups(groups)
                .connection_options(String::from("Cisco"), ConnectionOptions::builder().build())
                .build();
            hosts.insert(name, host);
        }

        Ok(hosts)
    }

    #[test]
    fn test_host_new() {
        let host = Host::new();
        assert_eq!(host.hostname, None);
        assert_eq!(host.port, None);
        assert_eq!(host.username, None);
        assert_eq!(host.password, None);
        assert_eq!(host.platform, None);
        assert_eq!(host.groups, None);
        assert_eq!(host.data, None);
        assert_eq!(host.connection_options, None);
    }

    #[test]
    fn test_hosts_new() {
        let mut hosts = Hosts::new();

        // Add 10 hosts to the hosts map with dummy data
        for i in 1..=10 {
            let name = format!("host{i}.example.com");
            let host = Host::builder()
                .hostname(&name)
                .port(2200 + i as u16)
                .username(format!("user{i}"))
                .password(format!("password{i}"))
                .platform(if i % 2 == 0 { "linux" } else { "windows" })
                .data(Data(serde_json::json!(vec![format!(
                    "data for host {}",
                    i
                )])))
                .connection_options(
                    String::from("Juniper"),
                    ConnectionOptions::builder().build(),
                )
                .build();

            hosts.add_host(name, host);
        }
        assert_eq!(hosts.len(), 10);
    }

    #[test]
    fn test_build_hosts() {
        let hosts = create_dummy_hosts();
        assert_eq!(hosts.unwrap().len(), 10);
    }

    #[test]
    fn test_connection_options_builder() {
        let extras = Extras::new(serde_json::json!({ "env": "lab", "tier": "core" }));

        let options = ConnectionOptions::builder()
            .hostname("192.0.2.55")
            .port(830)
            .username("netconf")
            .password("secret")
            .platform("iosxr")
            .extras(extras.clone())
            .build();

        assert_eq!(options.hostname(), Some("192.0.2.55"));
        assert_eq!(options.port(), Some(830));
        assert_eq!(options.username(), Some("netconf"));
        assert_eq!(options.password(), Some("secret"));
        assert_eq!(options.platform(), Some("iosxr"));
        assert_eq!(options.extras(), Some(&extras));
    }

    #[test]
    fn test_parent_groups() {
        let groups = vec![
            "cisco".to_string(),
            "Juniper".to_string(),
            "arista".to_string(),
        ];
        let serialized = serde_json::to_string(&groups).unwrap();
        assert_eq!(serialized, "[\"cisco\",\"Juniper\",\"arista\"]");
        let mut deserialized: ParentGroups = serde_json::from_str(&serialized).unwrap();
        let mut expected = ParentGroups(groups);
        deserialized.sort();
        expected.sort();
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_parent_groups_deduplication() {
        // Test that duplicate groups are removed during deserialization
        let groups_with_duplicates = vec![
            "cisco".to_string(),
            "juniper".to_string(),
            "cisco".to_string(), // duplicate
            "arista".to_string(),
            "juniper".to_string(), // duplicate
            "cisco".to_string(),   // duplicate
        ];

        let serialized = serde_json::to_string(&groups_with_duplicates).unwrap();
        let deserialized: ParentGroups = serde_json::from_str(&serialized).unwrap();

        // Should only contain unique values in order of first occurrence
        assert_eq!(deserialized.len(), 3);
        assert_eq!(deserialized[0], "cisco");
        assert_eq!(deserialized[1], "juniper");
        assert_eq!(deserialized[2], "arista");
    }

    #[test]
    fn test_parent_groups_preserves_order() {
        // Test that the order of first occurrence is preserved
        let groups = vec![
            "zebra".to_string(),
            "apple".to_string(),
            "zebra".to_string(), // duplicate
            "banana".to_string(),
        ];

        let serialized = serde_json::to_string(&groups).unwrap();
        let deserialized: ParentGroups = serde_json::from_str(&serialized).unwrap();

        // Should preserve order of first occurrence
        assert_eq!(deserialized.len(), 3);
        assert_eq!(deserialized[0], "zebra");
        assert_eq!(deserialized[1], "apple");
        assert_eq!(deserialized[2], "banana");
    }

    /// Tests the ParentGroups deserialization with an error.
    ///
    /// The error message is expected to be "Groups should be an array of strings for use with `ParentGroups`"
    #[test]
    fn test_parent_groups_err() {
        let name = String::from("name");
        let deserialized: Result<ParentGroups, serde_json::Error> = serde_json::from_str(&name);
        match deserialized {
            Ok(_) => panic!("Expected an error, but deserialization succeeded"),
            Err(err) => {
                assert_eq!(
                    err.to_string(),
                    "Groups should be an array of strings for use with `ParentGroups`"
                );
            }
        }
    }

    #[test]
    fn test_inventory_builder_defaults() {
        let inventory = Inventory::builder().build();
        assert_eq!(inventory.hosts().len(), 0);
        assert!(inventory.groups().is_none());
        assert!(inventory.defaults().is_none());
        assert!(inventory.transform_function_options().is_none());
    }

    #[test]
    fn resolve_connection_params_uses_cache() {
        let defaults: Defaults = serde_json::from_value(serde_json::json!({
            "connection_options": {
                "netconf": {
                    "hostname": "default-netconf",
                    "port": 2001
                }
            }
        }))
        .expect("defaults should deserialize");

        let mut hosts = Hosts::new();
        hosts.add_host("router1.lab", Host::builder().hostname("host-host").build());

        let inventory = Inventory::builder()
            .hosts(hosts)
            .defaults(defaults)
            .connections(ConnectionManager::default())
            .build();

        assert_eq!(inventory.resolved_host_cache_len(), 0);
        assert_eq!(inventory.resolved_params_cache_len(), 0);
        let _ = inventory
            .resolve_connection_params("router1.lab", "netconf")
            .expect("resolved params should exist");
        assert_eq!(inventory.resolved_host_cache_len(), 1);
        assert_eq!(inventory.resolved_params_cache_len(), 1);
        let _ = inventory
            .resolve_connection_params("router1.lab", "netconf")
            .expect("resolved params should exist");
        assert_eq!(inventory.resolved_host_cache_len(), 1);
        assert_eq!(inventory.resolved_params_cache_len(), 1);
    }

    /// Tests the internal group resolution logic for proper merging and cycle detection.
    ///
    /// This test verifies two critical aspects of the `resolve_group_internal` method:
    ///
    /// 1. **Hierarchical Merging**: Tests that group inheritance correctly merges settings
    ///    from parent groups in the proper order. Creates a three-level hierarchy (a -> b -> c)
    ///    and verifies that settings cascade properly, with child groups overriding parent
    ///    settings as expected.
    ///
    /// 2. **Cycle Detection**: Tests that circular group references are detected and handled
    ///    gracefully without causing infinite loops. Creates a two-group cycle (cycle-a -> cycle-b -> cycle-a)
    ///    and verifies that resolution completes successfully without hanging.
    ///
    /// # Test Structure
    ///
    /// The test is divided into two parts:
    ///
    /// ## Part 1: Hierarchical Merging
    /// - Creates groups with inheritance chain: a (username) -> b (hostname, port) -> c (port, platform)
    /// - Resolves group "a" and verifies all inherited settings are present
    /// - Validates that closer ancestors override more distant ones (b's port overrides c's port)
    ///
    /// ## Part 2: Cycle Detection
    /// - Creates a circular reference: cycle-a -> cycle-b -> cycle-a
    /// - Attempts to resolve cycle-a
    /// - Verifies that resolution completes without infinite loop
    /// - Confirms that the resolved group maintains its structure
    ///
    /// # Assertions
    ///
    /// For hierarchical merging:
    /// - `hostname` should be "b-host" (from group b)
    /// - `port` should be 2002 (from group b, overriding c's 2001)
    /// - `platform` should be "linux" (from group c)
    /// - `username` should be "a-user" (from group a)
    ///
    /// For cycle detection:
    /// - Resolution should complete without panic or timeout
    /// - Resolved group should maintain its groups field
    #[test]
    fn resolve_group_internal_merges_and_detects_cycle() {
        let mut groups = Groups::new();

        let mut a_parents = ParentGroups::new();
        a_parents.push("b".to_string());
        let group_a = Group::builder()
            .username("a-user")
            .groups(a_parents)
            .build();

        let mut b_parents = ParentGroups::new();
        b_parents.push("c".to_string());
        let group_b = Group::builder()
            .hostname("b-host")
            .port(2002)
            .groups(b_parents)
            .build();

        let group_c = Group::builder().port(2001).platform("linux").build();

        groups.add_group("a", group_a);
        groups.add_group("b", group_b);
        groups.add_group("c", group_c);

        let inventory = Inventory::builder().groups(groups).build();

        let mut stack = std::collections::HashSet::new();
        let mut memo = std::collections::HashMap::new();
        let resolved = inventory
            .resolve_group_internal("a", &mut stack, &mut memo)
            .expect("group should resolve");

        assert_eq!(resolved.hostname(), Some("b-host"));
        assert_eq!(resolved.port(), Some(2002));
        assert_eq!(resolved.platform(), Some("linux"));
        assert_eq!(resolved.username(), Some("a-user"));

        let mut cycle_groups = Groups::new();
        let mut c1 = ParentGroups::new();
        c1.push("cycle-b".to_string());
        let mut c2 = ParentGroups::new();
        c2.push("cycle-a".to_string());
        cycle_groups.add_group("cycle-a", Group::builder().groups(c1).build());
        cycle_groups.add_group("cycle-b", Group::builder().groups(c2).build());

        let cycle_inventory = Inventory::builder().groups(cycle_groups).build();
        let mut cycle_stack = std::collections::HashSet::new();
        let mut cycle_memo = std::collections::HashMap::new();
        let cycle_resolved = cycle_inventory
            .resolve_group_internal("cycle-a", &mut cycle_stack, &mut cycle_memo)
            .expect("cycle should not infinite loop");
        assert!(cycle_resolved.groups().is_some());
    }

    /// Tests that transform functions are applied to hosts and results are cached in HostsView.
    ///
    /// This test verifies two critical behaviors of the `HostsView` implementation:
    ///
    /// 1. **Transform Application**: Confirms that the configured transform function is
    ///    applied when accessing hosts through the view, modifying host properties as expected.
    ///
    /// 2. **Caching Behavior**: Validates that transformed results are cached, ensuring the
    ///    transform function is called only once per unique host regardless of how many times
    ///    that host is accessed.
    ///
    /// The test creates a transform function that increments each host's port number by 1
    /// and tracks the number of times it's called. It then verifies that:
    /// - Multiple accesses to the same host return the same transformed result
    /// - The transform is only called once per host (cached after first access)
    /// - Iteration over the view also uses the cache when available
    ///
    /// # Test Structure
    ///
    /// 1. Creates a transform function that increments port numbers and counts invocations
    /// 2. Builds an inventory with two hosts (ports 10 and 20)
    /// 3. Accesses the same host twice via `get()` and verifies caching
    /// 4. Iterates over all hosts and verifies the transform was called exactly twice total
    ///
    /// # Assertions
    ///
    /// - First access to "h1" returns port 11 (10 + 1)
    /// - Second access to "h1" returns port 11 (cached, transform not called again)
    /// - Transform function called exactly once after two accesses to same host
    /// - Transform function called exactly twice total after iterating all hosts
    #[test]
    fn transform_and_cache_hosts_view() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = Arc::clone(&calls);
        let transform = TransformFunction::new(move |host, _| {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            host.to_builder().port(host.port().unwrap_or(0) + 1).build()
        });

        let mut hosts = Hosts::new();
        hosts.add_host("h1", Host::builder().port(10).build());
        hosts.add_host("h2", Host::builder().port(20).build());

        let inventory = Inventory::builder()
            .hosts(hosts)
            .transform_function(transform)
            .build();

        let view = inventory.hosts();
        let first = view.get("h1").expect("host exists");
        let second = view.get("h1").expect("host exists");
        assert_eq!(first.port(), Some(11));
        assert_eq!(second.port(), Some(11));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // Iterate over all hosts which triggers the transform function again
        // as h2 has not been accessed yet. The transform function should be called twice.
        let _ = view.iter().collect::<Vec<_>>();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn hosts_view_respects_scope_without_touching_out_of_scope_cache() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = Arc::clone(&calls);
        let transform = TransformFunction::new(move |host, _| {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            host.to_builder().port(host.port().unwrap_or(0) + 1).build()
        });

        let mut hosts = Hosts::new();
        hosts.add_host("h1", Host::builder().port(10).build());
        hosts.add_host("h2", Host::builder().port(20).build());

        let inventory = Inventory::builder()
            .hosts(hosts)
            .transform_function(transform)
            .build();
        inventory.state().mark_failed("h2");

        let view = inventory.hosts();
        assert_eq!(view.len(), 1);
        assert_eq!(view.keys().count(), 1);
        assert!(view.get("h2").is_none());

        let only_host = view.get("h1").expect("in-scope host exists");
        assert_eq!(only_host.port(), Some(11));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let iterated = view.iter().collect::<Vec<_>>();
        assert_eq!(iterated.len(), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    /// Tests the `merge_data` function's behavior when merging JSON objects and replacing non-objects.
    ///
    /// This test verifies two critical behaviors of the `merge_data` function:
    ///
    /// 1. **Object Merging**: When both target and source are JSON objects, the function should
    ///    merge their key-value pairs. Keys present in both objects should be overwritten with
    ///    the source's values, while keys unique to either object should be preserved.
    ///
    /// 2. **Non-Object Replacement**: When the target is not a JSON object (e.g., an array) but
    ///    the source is an object, the entire target should be replaced with the source object
    ///    rather than attempting to merge incompatible types.
    ///
    /// # Test Structure
    ///
    /// The test is divided into two parts:
    ///
    /// ## Part 1: Object Merging
    /// - Creates a target object with keys "a" and "b"
    /// - Creates a source object with keys "b" and "c"
    /// - Merges source into target
    /// - Verifies that:
    ///   - Key "a" retains its original value (1)
    ///   - Key "b" is overwritten with source's value (3)
    ///   - Key "c" is added from source (4)
    ///
    /// ## Part 2: Non-Object Replacement
    /// - Creates a target that is a JSON array
    /// - Creates a source that is a JSON object
    /// - Merges source into target
    /// - Verifies that the target is completely replaced and is now an object
    ///
    /// # Assertions
    ///
    /// For object merging:
    /// - `target["a"]` should equal 1 (preserved from original)
    /// - `target["b"]` should equal 3 (overwritten by source)
    /// - `target["c"]` should equal 4 (added from source)
    ///
    /// For non-object replacement:
    /// - Target should be a JSON object after merge (not an array)
    #[test]
    fn merge_data_merges_objects_and_replaces_non_objects() {
        let mut target = Some(Data::new(serde_json::json!({
            "a": 1,
            "b": 2
        })));
        let source = Some(Data::new(serde_json::json!({
            "b": 3,
            "c": 4
        })));

        merge_data(&mut target, &source);
        let target_obj = target
            .as_ref()
            .and_then(|data| data.as_object())
            .expect("target should be object");
        assert_eq!(target_obj.get("a").and_then(|v| v.as_i64()), Some(1));
        assert_eq!(target_obj.get("b").and_then(|v| v.as_i64()), Some(3));
        assert_eq!(target_obj.get("c").and_then(|v| v.as_i64()), Some(4));

        let mut non_object_target = Some(Data::new(serde_json::json!(["x", "y"])));
        let object_source = Some(Data::new(serde_json::json!({ "k": "v" })));
        merge_data(&mut non_object_target, &object_source);
        assert!(non_object_target
            .as_ref()
            .and_then(|data| data.as_object())
            .is_some());
    }

    #[test]
    fn merge_connection_options_fields_overrides_only_present_values() {
        let mut target = ConnectionOptions::builder()
            .hostname("primary")
            .port(22)
            .username("user")
            .platform("linux")
            .build();

        let source = ConnectionOptions::builder()
            .port(2222)
            .password("secret")
            .extras(Extras::new(serde_json::json!({ "tier": "core" })))
            .build();

        merge_connection_options_fields(&mut target, &source);

        assert_eq!(target.hostname(), Some("primary"));
        assert_eq!(target.port(), Some(2222));
        assert_eq!(target.username(), Some("user"));
        assert_eq!(target.password(), Some("secret"));
        assert_eq!(target.platform(), Some("linux"));
        assert_eq!(
            target
                .extras()
                .and_then(|v| v.get("tier"))
                .and_then(|v| v.as_str()),
            Some("core")
        );
    }

    #[test]
    fn base_methods_schema_returns_json() {
        let schema = Host::schema();
        let parsed: serde_json::Value =
            serde_json::from_str(&schema).expect("schema should be valid JSON");
        assert!(parsed.get("$schema").is_some());
    }

    #[test]
    fn connection_options_default_and_to_builder_round_trip() {
        let options = ConnectionOptions::default();
        assert_eq!(options.hostname(), None);
        assert_eq!(options.port(), None);
        assert_eq!(options.username(), None);
        assert_eq!(options.password(), None);
        assert_eq!(options.platform(), None);
        assert_eq!(options.extras(), None);

        let rebuilt = options.to_builder().build();
        assert_eq!(options, rebuilt);
    }

    #[test]
    fn defaults_builder_to_builder_and_accessors() {
        let defaults = Defaults::builder()
            .hostname("default-host")
            .port(22)
            .username("admin")
            .password("secret")
            .platform("linux")
            .data(Data::new(serde_json::json!({"env": "lab"})))
            .build();

        assert!(!defaults.is_empty());
        assert_eq!(defaults.hostname(), Some("default-host"));
        assert_eq!(defaults.port(), Some(22));
        assert_eq!(defaults.username(), Some("admin"));
        assert_eq!(defaults.password(), Some("secret"));
        assert_eq!(defaults.platform(), Some("linux"));
        assert!(defaults.data().is_some());

        let modified = defaults.to_builder().port(2222).build();
        assert_eq!(modified.port(), Some(2222));
        assert_eq!(modified.username(), Some("admin"));
    }

    #[test]
    fn host_and_group_to_builder_preserve_connection_options() {
        let opts1 = ConnectionOptions::builder().port(22).build();
        let opts2 = ConnectionOptions::builder().port(830).build();

        let host = Host::builder()
            .hostname("h1")
            .connection_options("ssh", opts1.clone())
            .connection_options("netconf", opts2.clone())
            .build();
        let host_round = host.to_builder().build();
        assert_eq!(
            host_round
                .connection_options()
                .and_then(|m| m.get("ssh"))
                .and_then(|o| o.port()),
            Some(22)
        );

        let group = Group::builder()
            .hostname("g1")
            .connection_options("ssh", opts1)
            .build();
        let group_round = group.to_builder().build();
        assert_eq!(
            group_round
                .connection_options()
                .and_then(|m| m.get("ssh"))
                .and_then(|o| o.port()),
            Some(22)
        );
    }

    #[test]
    fn groups_default_is_empty() {
        let groups = Groups::default();
        assert!(groups.is_empty());
    }

    #[test]
    fn transform_function_group_and_defaults_methods() {
        struct CountTransform {
            group_calls: Arc<AtomicUsize>,
            defaults_calls: Arc<AtomicUsize>,
        }

        impl Transform for CountTransform {
            fn transform_group(
                &self,
                group: &Group,
                _options: Option<&TransformFunctionOptions>,
            ) -> Group {
                self.group_calls.fetch_add(1, Ordering::SeqCst);
                group.to_builder().port(443).build()
            }

            fn transform_defaults(
                &self,
                defaults: &Defaults,
                _options: Option<&TransformFunctionOptions>,
            ) -> Defaults {
                self.defaults_calls.fetch_add(1, Ordering::SeqCst);
                defaults.to_builder().username("admin").build()
            }
        }

        let group_calls = Arc::new(AtomicUsize::new(0));
        let defaults_calls = Arc::new(AtomicUsize::new(0));
        let transform = TransformFunction::new_full(CountTransform {
            group_calls: Arc::clone(&group_calls),
            defaults_calls: Arc::clone(&defaults_calls),
        });

        let group = Group::builder().platform("linux").build();
        let defaults = Defaults::builder().port(22).build();

        let transformed_group = transform.transform_group(&group, None);
        let transformed_defaults = transform.transform_defaults(&defaults, None);

        assert_eq!(transformed_group.port(), Some(443));
        assert_eq!(transformed_defaults.username(), Some("admin"));
        assert_eq!(group_calls.load(Ordering::SeqCst), 1);
        assert_eq!(defaults_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn transform_function_options_new() {
        let options = TransformFunctionOptions::new(serde_json::json!({"k": "v"}));
        assert_eq!(options.get("k").and_then(|v| v.as_str()), Some("v"));
    }

    #[test]
    fn connection_manager_insert_get_and_close() {
        #[derive(Debug)]
        struct TestConnection {
            closes: Arc<AtomicUsize>,
            key: ConnectionKey,
        }

        #[async_trait]
        impl Connection for TestConnection {
            fn create(&self, key: &ConnectionKey) -> Box<dyn Connection> {
                Box::new(TestConnection {
                    closes: Arc::clone(&self.closes),
                    key: key.clone(),
                })
            }

            fn is_alive(&self) -> bool {
                true
            }

            async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
                Ok(())
            }

            fn close(&mut self) -> ConnectionKey {
                self.closes.fetch_add(1, Ordering::SeqCst);
                self.key.clone()
            }
        }

        let manager = ConnectionManager::default();
        let closes = Arc::new(AtomicUsize::new(0));
        let key = ConnectionKey::new("h1", "ssh");
        let connection = Arc::new(Mutex::new(TestConnection {
            closes: Arc::clone(&closes),
            key: key.clone(),
        })) as Arc<Mutex<dyn Connection>>;

        manager.insert(key.clone(), Arc::clone(&connection));
        let fetched = manager.get(&key).expect("connection should exist");
        assert!(Arc::ptr_eq(&connection, &fetched));

        manager.close_connection(&key);
        assert_eq!(closes.load(Ordering::SeqCst), 1);
        assert!(manager.get(&key).is_none());

        let key2 = ConnectionKey::new("h2", "netconf");
        let connection2 = Arc::new(Mutex::new(TestConnection {
            closes: Arc::clone(&closes),
            key: key2.clone(),
        })) as Arc<Mutex<dyn Connection>>;
        manager.insert(key.clone(), connection);
        manager.insert(key2.clone(), connection2);
        manager.close_all_connections();
        assert_eq!(closes.load(Ordering::SeqCst), 3);
        assert!(manager.get(&key).is_none());
        assert!(manager.get(&key2).is_none());
    }

    #[test]
    fn inventory_connections_and_resolve_host_cache_hit() {
        let mut hosts = Hosts::new();
        hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
        let inventory = Inventory::builder().hosts(hosts).build();

        let _ = inventory.connections();
        assert_eq!(inventory.resolved_host_cache_len(), 0);
        let _ = inventory.resolve_host("router1");
        assert_eq!(inventory.resolved_host_cache_len(), 1);
        let _ = inventory.resolve_host("router1");
        assert_eq!(inventory.resolved_host_cache_len(), 1);
    }

    #[test]
    fn groups_view_applies_transform_and_accessors_work() {
        let group_calls = Arc::new(AtomicUsize::new(0));
        let group_calls_clone = Arc::clone(&group_calls);
        struct GroupOnlyTransform {
            calls: Arc<AtomicUsize>,
        }

        impl Transform for GroupOnlyTransform {
            fn transform_group(
                &self,
                group: &Group,
                _options: Option<&TransformFunctionOptions>,
            ) -> Group {
                self.calls.fetch_add(1, Ordering::SeqCst);
                group.to_builder().port(1234).build()
            }
        }

        let transform = TransformFunction::new_full(GroupOnlyTransform {
            calls: group_calls_clone,
        });

        let mut groups = Groups::new();
        groups.add_group("core", Group::builder().platform("linux").build());
        groups.add_group("edge", Group::builder().platform("linux").build());

        let inventory = Inventory::builder()
            .groups(groups)
            .transform_function(transform)
            .build();

        let view = inventory.groups().expect("groups view exists");
        assert_eq!(view.len(), 2);
        assert!(!view.is_empty());
        assert_eq!(view.keys().count(), 2);

        let core = view.get("core").expect("group exists");
        assert_eq!(core.port(), Some(1234));

        let _ = view.iter().collect::<Vec<_>>();
        assert_eq!(group_calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn merge_connection_options_merges_maps_and_overrides_fields() {
        let mut target: Option<CustomTreeMap<ConnectionOptions>> = Some(CustomTreeMap::new());
        let mut source: Option<CustomTreeMap<ConnectionOptions>> = Some(CustomTreeMap::new());

        if let Some(map) = target.as_mut() {
            map.insert(
                "ssh",
                ConnectionOptions::builder().hostname("t").port(22).build(),
            );
        }
        if let Some(map) = source.as_mut() {
            map.insert(
                "ssh",
                ConnectionOptions::builder()
                    .port(2222)
                    .username("u")
                    .build(),
            );
            map.insert("netconf", ConnectionOptions::builder().port(830).build());
        }

        merge_connection_options(&mut target, &source);
        let merged = target.expect("target should exist");
        let ssh = merged.get("ssh").expect("ssh should exist");
        assert_eq!(ssh.hostname(), Some("t"));
        assert_eq!(ssh.port(), Some(2222));
        assert_eq!(ssh.username(), Some("u"));
        assert!(merged.get("netconf").is_some());
    }

    #[test]
    fn resolve_host_applies_defaults_groups_then_host_in_order() {
        let defaults = Defaults::builder().port(10).build();

        let group_a = Group::builder().port(20).build();
        let group_b = Group::builder().port(30).build();

        let mut groups = Groups::new();
        groups.add_group("a", group_a);
        groups.add_group("b", group_b);

        let mut hosts = Hosts::new();
        let mut parents = ParentGroups::new();
        parents.push("a".to_string());
        parents.push("b".to_string());
        let host = Host::builder().port(40).groups(parents).build();
        hosts.add_host("h1", host);

        let inventory = Inventory::builder()
            .hosts(hosts)
            .groups(groups)
            .defaults(defaults)
            .build();

        let resolved = inventory.resolve_host("h1").expect("host should resolve");
        assert_eq!(resolved.port(), Some(40));
    }

    #[test]
    fn resolve_connection_params_applies_priority_order() {
        let defaults = Defaults::builder()
            .port(10)
            .connection_options("ssh", ConnectionOptions::builder().port(11).build())
            .build();

        let group = Group::builder()
            .port(20)
            .connection_options("ssh", ConnectionOptions::builder().port(21).build())
            .build();

        let mut groups = Groups::new();
        groups.add_group("g1", group);

        let mut parents = ParentGroups::new();
        parents.push("g1".to_string());
        let host = Host::builder()
            .port(30)
            .groups(parents)
            .connection_options("ssh", ConnectionOptions::builder().port(31).build())
            .build();

        let mut hosts = Hosts::new();
        hosts.add_host("h1", host);

        let inventory = Inventory::builder()
            .hosts(hosts)
            .groups(groups)
            .defaults(defaults)
            .build();

        let resolved = inventory
            .resolve_connection_params("h1", "ssh")
            .expect("params should resolve");
        assert_eq!(resolved.port, Some(31));
    }

    #[test]
    fn connection_manager_get_or_create_errors_without_factory() {
        let manager = ConnectionManager::default();
        let key = ConnectionKey::new("h1", "ssh");
        let err = manager.get_or_create(key).unwrap_err();
        assert_eq!(err, "connection factory not set");
    }

    #[test]
    fn connection_manager_open_connection_propagates_open_error() {
        #[derive(Debug)]
        struct FailingConnection;

        #[async_trait]
        impl Connection for FailingConnection {
            fn create(&self, _key: &ConnectionKey) -> Box<dyn Connection> {
                Box::new(FailingConnection)
            }

            fn is_alive(&self) -> bool {
                false
            }

            async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
                Err("boom".to_string())
            }

            fn close(&mut self) -> ConnectionKey {
                ConnectionKey::new("h1", "ssh")
            }
        }

        let factory = Arc::new(|_key: &ConnectionKey| {
            Some(Arc::new(Mutex::new(FailingConnection)) as Arc<Mutex<dyn Connection>>)
        });
        let manager = ConnectionManager::with_connection_factory(factory);
        let key = ConnectionKey::new("h1", "ssh");
        let params = ResolvedConnectionParams {
            hostname: "h1".to_string(),
            port: Some(22),
            username: None,
            password: None,
            platform: None,
            extras: None,
        };

        let err = run_async(manager.open_connection(&key, &params)).unwrap_err();
        assert_eq!(err, "boom");
        let counters = manager.connection_counters_for("ssh").unwrap();
        assert_eq!(counters.create_calls, 1);
        assert_eq!(counters.open_calls, 0);
    }

    #[test]
    fn connection_manager_open_connection_returns_none_when_factory_returns_none() {
        let factory = Arc::new(|_key: &ConnectionKey| None);
        let manager = ConnectionManager::with_connection_factory(factory);
        let key = ConnectionKey::new("h1", "ssh");
        let params = ResolvedConnectionParams {
            hostname: "h1".to_string(),
            port: Some(22),
            username: None,
            password: None,
            platform: None,
            extras: None,
        };

        let result = run_async(manager.open_connection(&key, &params)).unwrap();
        assert!(result.is_none());
        assert!(manager.connection_counters_for("ssh").is_none());
    }

    #[test]
    fn inventory_applies_transform_options_to_hosts_groups_and_defaults() {
        struct OptTransform;

        impl Transform for OptTransform {
            fn transform_host(
                &self,
                host: &Host,
                options: Option<&TransformFunctionOptions>,
            ) -> Host {
                let port = options
                    .and_then(|opts| opts.get("port").and_then(|v| v.as_u64()))
                    .unwrap_or(0) as u16;
                host.to_builder().port(port).build()
            }

            fn transform_group(
                &self,
                group: &Group,
                options: Option<&TransformFunctionOptions>,
            ) -> Group {
                let username = options
                    .and_then(|opts| opts.get("username").and_then(|v| v.as_str()))
                    .unwrap_or("default");
                group.to_builder().username(username).build()
            }

            fn transform_defaults(
                &self,
                defaults: &Defaults,
                options: Option<&TransformFunctionOptions>,
            ) -> Defaults {
                let hostname = options
                    .and_then(|opts| opts.get("hostname").and_then(|v| v.as_str()))
                    .unwrap_or("defaults");
                defaults.to_builder().hostname(hostname).build()
            }
        }

        let transform = TransformFunction::new_full(OptTransform);
        let options = TransformFunctionOptions::new(serde_json::json!({
            "port": 2022,
            "username": "opt-user",
            "hostname": "opt-defaults"
        }));

        let mut hosts = Hosts::new();
        hosts.add_host("h1", Host::builder().build());

        let mut groups = Groups::new();
        groups.add_group("g1", Group::builder().build());

        let defaults = Defaults::builder().build();

        let inventory = Inventory::builder()
            .hosts(hosts)
            .groups(groups)
            .defaults(defaults)
            .transform_function(transform)
            .transform_function_options(options)
            .build();

        let host = inventory.hosts().get("h1").expect("host exists");
        assert_eq!(host.port(), Some(2022));

        let group = inventory.groups().expect("groups exist").get("g1").unwrap();
        assert_eq!(group.username(), Some("opt-user"));

        let defaults = inventory.defaults().expect("defaults exist");
        assert_eq!(defaults.hostname(), Some("opt-defaults"));
    }

    #[test]
    fn host_deserializes_with_missing_fields_as_none() {
        let host: Host = serde_json::from_value(serde_json::json!({}))
            .expect("host should deserialize from empty object");
        assert_eq!(host.hostname(), None);
        assert_eq!(host.port(), None);
        assert_eq!(host.username(), None);
        assert_eq!(host.password(), None);
        assert_eq!(host.platform(), None);
        assert!(host.groups().is_none());
        assert!(host.data().is_none());
        assert!(host.connection_options().is_none());
    }
}
