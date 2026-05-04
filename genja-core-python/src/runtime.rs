use ::genja::Genja as RuntimeGenja;
use ::genja_core::inventory::{Hosts, Inventory};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use std::sync::Mutex;

use crate::plugin_manager::{register_python_plugin_on_manager, PyPluginManager};
use crate::settings::PySettings;
use crate::task::{self, PyTaskResults};

#[pyclass(name = "Genja")]
#[derive(Clone)]
pub struct PyGenja {
    inner: RuntimeGenja,
}

#[pymethods]
impl PyGenja {
    #[staticmethod]
    #[pyo3(signature = (hosts, settings=None, plugin_manager=None))]
    fn builder(
        hosts: Bound<'_, PyAny>,
        settings: Option<PyRef<'_, PySettings>>,
        plugin_manager: Option<PyRef<'_, PyPluginManager>>,
    ) -> PyResult<PyGenjaBuilder> {
        let inventory = python_hosts_to_inventory(hosts)?;
        let settings = settings.map(|settings| settings.inner.clone());
        let plugin_manager = if let Some(plugin_manager) = plugin_manager {
            plugin_manager.take_inner()?
        } else {
            PyPluginManager::new().take_inner()?
        };

        Ok(PyGenjaBuilder {
            inner: Mutex::new(Some(PyGenjaBuilderState {
                inventory,
                settings,
                plugin_manager,
                runner: None,
            })),
        })
    }

    #[staticmethod]
    #[pyo3(signature = (hosts, settings=None, plugin_manager=None))]
    fn from_hosts(
        hosts: Bound<'_, PyAny>,
        settings: Option<PyRef<'_, PySettings>>,
        plugin_manager: Option<PyRef<'_, PyPluginManager>>,
    ) -> PyResult<Self> {
        let builder = Self::builder(hosts, settings, plugin_manager)?;
        builder.build()
    }

    #[staticmethod]
    fn from_settings_file(path: &str) -> PyResult<Self> {
        let inner = RuntimeGenja::from_settings_file(path).map_err(|err| {
            PyValueError::new_err(format!(
                "failed to build Genja runtime from settings file {path}: {err}"
            ))
        })?;
        Ok(Self { inner })
    }

    fn with_runner(&self, runner: &str) -> PyResult<Self> {
        let inner = self.inner.with_runner(runner).map_err(|err| {
            PyValueError::new_err(format!("failed to select runner {runner}: {err}"))
        })?;
        Ok(Self { inner })
    }

    fn filter_by_key(&self, key: &str) -> PyResult<Self> {
        let inner = self.inner.filter_by_key(key).map_err(|err| {
            PyValueError::new_err(format!("failed to filter hosts by key {key}: {err}"))
        })?;
        Ok(Self { inner })
    }

    fn filter_by_key_value(&self, key: &str, value_pattern: &str) -> PyResult<Self> {
        let inner = self
            .inner
            .filter_by_key_value(key, value_pattern)
            .map_err(|err| {
                PyValueError::new_err(format!(
                    "failed to filter hosts by key {key} and value pattern {value_pattern}: {err}"
                ))
            })?;
        Ok(Self { inner })
    }

    fn inventory(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let inventory = self.inner.inventory().map_err(|err| {
            PyValueError::new_err(format!("failed to access loaded inventory: {err}"))
        })?;
        inventory_hosts_to_py_dict(py, inventory.hosts_raw())
    }

    fn iter_inventory_hosts(&self, py: Python<'_>) -> PyResult<Vec<(String, Py<PyAny>)>> {
        let hosts = self.inner.iter_inventory_hosts().map_err(|err| {
            PyValueError::new_err(format!("failed to iterate inventory hosts: {err}"))
        })?;
        hosts.into_iter()
            .map(|(host_id, host)| {
                Ok((
                    host_id.to_string(),
                    task::host_to_py_dict(py, &host)?.into_any().unbind(),
                ))
            })
            .collect()
    }

    fn hosts_raw(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let inventory = self.inner.inventory().map_err(|err| {
            PyValueError::new_err(format!("failed to access loaded inventory: {err}"))
        })?;
        inventory_hosts_to_py_dict(py, inventory.hosts_raw())
    }

    #[pyo3(signature = (task_class, max_depth=None))]
    fn run_task(
        &self,
        py: Python<'_>,
        task_class: Bound<'_, PyAny>,
        max_depth: Option<usize>,
    ) -> PyResult<PyTaskResults> {
        task::run_task(py, &self.inner, task_class, max_depth)
    }

    fn __repr__(&self) -> String {
        format!(
            "Genja(plugins_loaded={}, inventory_loaded={})",
            self.inner.plugins_loaded(),
            self.inner.inventory_loaded()
        )
    }
}

struct PyGenjaBuilderState {
    inventory: Inventory,
    settings: Option<genja_core::Settings>,
    plugin_manager: genja_plugin_manager::PluginManager,
    runner: Option<String>,
}

#[pyclass(name = "GenjaBuilder")]
pub struct PyGenjaBuilder {
    inner: Mutex<Option<PyGenjaBuilderState>>,
}

#[pymethods]
impl PyGenjaBuilder {
    fn with_plugin(&self, plugin: Bound<'_, PyAny>) -> PyResult<Self> {
        let mut state = self.take_state()?;
        register_python_plugin_on_manager(&mut state.plugin_manager, plugin.unbind())?;
        Ok(Self {
            inner: Mutex::new(Some(state)),
        })
    }

    fn with_plugin_manager(&self, plugin_manager: PyRef<'_, PyPluginManager>) -> PyResult<Self> {
        let mut state = self.take_state()?;
        state.plugin_manager = plugin_manager.take_inner()?;
        Ok(Self {
            inner: Mutex::new(Some(state)),
        })
    }

    fn with_runner(&self, runner: &str) -> PyResult<Self> {
        let mut state = self.take_state()?;
        state.runner = Some(runner.to_string());
        Ok(Self {
            inner: Mutex::new(Some(state)),
        })
    }

    fn build(&self) -> PyResult<PyGenja> {
        let state = self.take_state()?;
        build_runtime(
            state.inventory,
            state.settings,
            state.plugin_manager,
            state.runner.as_deref(),
        )
    }

    fn __repr__(&self) -> String {
        match self.lock_inner() {
            Ok(guard) => {
                let consumed = guard.is_none();
                let runner = guard
                    .as_ref()
                    .and_then(|state| state.runner.as_deref())
                    .unwrap_or("None");
                format!("GenjaBuilder(consumed={consumed}, runner={runner})")
            }
            Err(_) => "GenjaBuilder(<unavailable>)".to_string(),
        }
    }
}

impl PyGenjaBuilder {
    fn lock_inner(&self) -> PyResult<std::sync::MutexGuard<'_, Option<PyGenjaBuilderState>>> {
        self.inner
            .lock()
            .map_err(|_| PyValueError::new_err("genja builder lock is poisoned"))
    }

    fn take_state(&self) -> PyResult<PyGenjaBuilderState> {
        let mut guard = self.lock_inner()?;
        guard
            .take()
            .ok_or_else(|| PyValueError::new_err("genja builder has already been consumed"))
    }
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyGenja>()?;
    module.add_class::<PyGenjaBuilder>()?;
    Ok(())
}

fn inventory_hosts_to_py_dict(py: Python<'_>, hosts: &Hosts) -> PyResult<Py<PyAny>> {
    let payload = PyDict::new(py);
    for (host_id, host) in hosts.iter() {
        payload.set_item(host_id.as_str(), task::host_to_py_dict(py, &host)?)?;
    }
    Ok(payload.into_any().unbind())
}

fn python_hosts_to_inventory(obj: Bound<'_, PyAny>) -> PyResult<Inventory> {
    let dict = obj.downcast::<PyDict>().map_err(|_| {
        PyValueError::new_err("hosts must be a dict mapping host id to host payload")
    })?;

    let mut hosts = Hosts::new();
    for (host_id, host_obj) in dict.iter() {
        let host_id: String = host_id.extract()?;
        let host = task::python_host_to_rust_host(host_obj)?;
        hosts.add_host(host_id, host);
    }

    Ok(Inventory::builder().hosts(hosts).build())
}

fn build_runtime(
    inventory: Inventory,
    settings: Option<genja_core::Settings>,
    plugin_manager: genja_plugin_manager::PluginManager,
    runner: Option<&str>,
) -> PyResult<PyGenja> {
    let mut builder = RuntimeGenja::builder(inventory).with_plugin_manager(plugin_manager);
    if let Some(settings) = settings {
        builder = builder.with_settings(settings);
    }
    let mut inner = builder
        .build()
        .map_err(|err| PyValueError::new_err(format!("failed to build Genja runtime: {err}")))?;
    if let Some(runner) = runner {
        inner = inner.with_runner(runner).map_err(|err| {
            PyValueError::new_err(format!("failed to select runner {runner}: {err}"))
        })?;
    }
    Ok(PyGenja { inner })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_manager::PyPluginManager;
    use pyo3::types::PyString;
    use std::sync::Once;

    fn init_python() {
        static INIT: Once = Once::new();
        INIT.call_once(pyo3::prepare_freethreaded_python);
    }

    #[test]
    fn python_hosts_to_inventory_converts_host_dict() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);

            let router1 = PyDict::new(py);
            router1.set_item("hostname", "10.0.0.1").unwrap();
            router1.set_item("platform", "ios").unwrap();

            let router2 = PyDict::new(py);
            router2.set_item("hostname", "10.0.0.2").unwrap();
            router2.set_item("port", 2222).unwrap();
            router2.set_item("platform", "nxos").unwrap();

            hosts.set_item("router1", router1).unwrap();
            hosts.set_item("router2", router2).unwrap();

            let inventory =
                python_hosts_to_inventory(hosts.into_any()).expect("hosts should convert");
            let inventory_hosts = inventory.hosts();

            assert_eq!(inventory_hosts.len(), 2);
            assert_eq!(
                inventory_hosts
                    .get("router1")
                    .expect("router1 should exist")
                    .hostname(),
                Some("10.0.0.1")
            );
            assert_eq!(
                inventory_hosts
                    .get("router2")
                    .expect("router2 should exist")
                    .port(),
                Some(2222)
            );
        });
    }

    #[test]
    fn python_hosts_to_inventory_rejects_non_dict_input() {
        init_python();
        Python::with_gil(|py| {
            let not_a_dict = PyString::new(py, "not-a-dict");

            let err = python_hosts_to_inventory(not_a_dict.into_any())
                .err()
                .expect("non-dict input should fail");
            assert!(err
                .to_string()
                .contains("hosts must be a dict mapping host id to host payload"));
        });
    }

    #[test]
    fn py_genja_from_hosts_builds_runtime() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);
            let router = PyDict::new(py);
            router.set_item("hostname", "10.0.0.1").unwrap();
            router.set_item("platform", "ios").unwrap();
            hosts.set_item("router1", router).unwrap();

            let runtime =
                PyGenja::from_hosts(hosts.into_any(), None, None).expect("runtime should build");

            assert!(runtime.inner.plugins_loaded());
            assert!(runtime.inner.inventory_loaded());
            assert!(runtime.__repr__().contains("Genja("));
        });
    }

    #[test]
    fn py_genja_builder_builds_runtime_with_runner() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);
            let router = PyDict::new(py);
            router.set_item("hostname", "10.0.0.1").unwrap();
            router.set_item("platform", "ios").unwrap();
            hosts.set_item("router1", router).unwrap();

            let builder =
                PyGenja::builder(hosts.into_any(), None, None).expect("builder should be created");
            let builder = builder
                .with_runner("serial")
                .expect("runner should be set on builder");
            let runtime = builder.build().expect("builder should produce runtime");

            assert!(runtime.inner.inventory_loaded());
            assert!(runtime.inner.get_runner_plugin("serial").is_ok());
        });
    }

    #[test]
    fn py_genja_from_hosts_accepts_plugin_manager() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);
            let router = PyDict::new(py);
            router.set_item("hostname", "10.0.0.1").unwrap();
            router.set_item("platform", "ios").unwrap();
            hosts.set_item("router1", router).unwrap();

            let plugin_manager =
                Py::new(py, PyPluginManager::new()).expect("plugin manager should be created");
            let plugin_manager_ref = plugin_manager.bind(py).borrow();

            let runtime = PyGenja::from_hosts(hosts.into_any(), None, Some(plugin_manager_ref))
                .expect("runtime should build with explicit plugin manager");

            assert!(runtime.inner.plugins_loaded());
            assert!(runtime.inner.inventory_loaded());
            assert!(runtime.inner.get_runner_plugin("serial").is_ok());
        });
    }

    #[test]
    fn py_genja_builder_consumes_previous_builder_instance() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);
            let router = PyDict::new(py);
            router.set_item("hostname", "10.0.0.1").unwrap();
            hosts.set_item("router1", router).unwrap();

            let builder =
                PyGenja::builder(hosts.into_any(), None, None).expect("builder should be created");
            let next_builder = builder
                .with_runner("serial")
                .expect("runner should be set on builder");
            let err = builder
                .build()
                .err()
                .expect("consumed builder should not build twice");
            assert!(err
                .to_string()
                .contains("genja builder has already been consumed"));
            assert!(next_builder.build().is_ok());
        });
    }

    #[test]
    fn py_genja_inventory_accessors_return_host_payloads() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);
            let router1 = PyDict::new(py);
            router1.set_item("hostname", "10.0.0.1").unwrap();
            router1.set_item("platform", "ios").unwrap();
            hosts.set_item("router1", router1).unwrap();

            let runtime =
                PyGenja::from_hosts(hosts.into_any(), None, None).expect("runtime should build");

            let inventory = runtime.inventory(py).expect("inventory accessor should work");
            let inventory: Bound<'_, PyDict> = inventory.bind(py).clone().downcast_into().unwrap();
            assert_eq!(
                inventory
                    .get_item("router1")
                    .unwrap()
                    .expect("router1 inventory host should exist")
                    .get_item("hostname")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "10.0.0.1"
            );

            let raw_hosts = runtime.hosts_raw(py).expect("hosts_raw accessor should work");
            let raw_hosts: Bound<'_, PyDict> = raw_hosts.bind(py).clone().downcast_into().unwrap();
            assert_eq!(
                raw_hosts
                    .get_item("router1")
                    .unwrap()
                    .expect("router1 raw host should exist")
                    .get_item("platform")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "ios"
            );

            let inventory_hosts = runtime
                .iter_inventory_hosts(py)
                .expect("iter_inventory_hosts should work");
            assert_eq!(inventory_hosts.len(), 1);
            assert_eq!(inventory_hosts[0].0, "router1");
            assert_eq!(
                inventory_hosts[0]
                    .1
                    .bind(py)
                    .get_item("hostname")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "10.0.0.1"
            );
        });
    }

    #[test]
    fn py_genja_filter_methods_return_filtered_runtime() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);

            let router1 = PyDict::new(py);
            router1.set_item("hostname", "10.0.0.1").unwrap();
            router1.set_item("platform", "ios").unwrap();
            let router1_data = PyDict::new(py);
            let router1_site = PyDict::new(py);
            router1_site.set_item("role", "core").unwrap();
            router1_data.set_item("site", router1_site).unwrap();
            router1.set_item("data", router1_data).unwrap();

            let router2 = PyDict::new(py);
            router2.set_item("hostname", "10.0.0.2").unwrap();
            router2.set_item("platform", "nxos").unwrap();
            let router2_data = PyDict::new(py);
            let router2_site = PyDict::new(py);
            router2_site.set_item("role", "edge").unwrap();
            router2_data.set_item("site", router2_site).unwrap();
            router2.set_item("data", router2_data).unwrap();

            hosts.set_item("router1", router1).unwrap();
            hosts.set_item("router2", router2).unwrap();

            let runtime =
                PyGenja::from_hosts(hosts.into_any(), None, None).expect("runtime should build");

            let filtered = runtime
                .filter_by_key_value("data.site.role", "^core$")
                .expect("filter_by_key_value should work");
            assert_eq!(filtered.inner.host_ids().len(), 1);
            assert_eq!(filtered.inner.host_ids()[0].as_str(), "router1");

            let key_filtered = runtime
                .filter_by_key("data.site.role")
                .expect("filter_by_key should work");
            assert_eq!(key_filtered.inner.host_ids().len(), 2);
        });
    }

    #[test]
    fn register_adds_genja_class_to_module() {
        init_python();
        Python::with_gil(|py| {
            let module =
                PyModule::new(py, "test_runtime_module").expect("test module should be created");

            register(&module).expect("runtime class should register");

            assert!(module.getattr("Genja").is_ok());
            assert!(module.getattr("GenjaBuilder").is_ok());
        });
    }
}
