use ::genja::Genja as RuntimeGenja;
use ::genja_core::inventory::{Hosts, Inventory};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

use crate::plugin_manager::PyPluginManager;
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
    fn from_hosts(
        hosts: Bound<'_, PyAny>,
        settings: Option<PyRef<'_, PySettings>>,
        plugin_manager: Option<PyRef<'_, PyPluginManager>>,
    ) -> PyResult<Self> {
        let inventory = python_hosts_to_inventory(hosts)?;
        let mut builder = RuntimeGenja::builder(inventory);
        if let Some(settings) = settings {
            builder = builder.with_settings(settings.inner.clone());
        }
        if let Some(plugin_manager) = plugin_manager {
            builder = builder.with_plugin_manager(plugin_manager.take_inner()?);
        }
        let inner = builder
            .build()
            .map_err(|err| PyValueError::new_err(format!("failed to build Genja runtime: {err}")))?;
        Ok(Self { inner })
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
        let inner = self
            .inner
            .with_runner(runner)
            .map_err(|err| PyValueError::new_err(format!("failed to select runner {runner}: {err}")))?;
        Ok(Self { inner })
    }

    #[pyo3(signature = (task_class, max_depth=None))]
    fn run_task(
        &self,
        task_class: Bound<'_, PyAny>,
        max_depth: Option<usize>,
    ) -> PyResult<PyTaskResults> {
        task::run_task(&self.inner, task_class, max_depth)
    }

    fn __repr__(&self) -> String {
        format!(
            "Genja(plugins_loaded={}, inventory_loaded={})",
            self.inner.plugins_loaded(),
            self.inner.inventory_loaded()
        )
    }
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyGenja>()?;
    Ok(())
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
            assert!(
                err.to_string()
                    .contains("hosts must be a dict mapping host id to host payload")
            );
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
    fn py_genja_from_hosts_accepts_plugin_manager() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);
            let router = PyDict::new(py);
            router.set_item("hostname", "10.0.0.1").unwrap();
            router.set_item("platform", "ios").unwrap();
            hosts.set_item("router1", router).unwrap();

            let plugin_manager = Py::new(py, PyPluginManager::new())
                .expect("plugin manager should be created");
            let plugin_manager_ref = plugin_manager.bind(py).borrow();

            let runtime = PyGenja::from_hosts(hosts.into_any(), None, Some(plugin_manager_ref))
                .expect("runtime should build with explicit plugin manager");

            assert!(runtime.inner.plugins_loaded());
            assert!(runtime.inner.inventory_loaded());
            assert!(runtime.inner.get_runner_plugin("serial").is_ok());
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
        });
    }
}
