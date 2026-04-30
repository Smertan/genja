use ::genja::Genja as RuntimeGenja;
use ::genja_core::inventory::{Hosts, Inventory};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

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
    #[pyo3(signature = (hosts, settings=None))]
    fn from_hosts(hosts: Bound<'_, PyAny>, settings: Option<PyRef<'_, PySettings>>) -> PyResult<Self> {
        let inventory = python_hosts_to_inventory(hosts)?;
        let inner = if let Some(settings) = settings {
            RuntimeGenja::builder(inventory)
                .with_settings(settings.inner.clone())
                .build()
                .map_err(|err| PyValueError::new_err(format!("failed to build Genja runtime: {err}")))?
        } else {
            RuntimeGenja::from_inventory(inventory)
        };
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
