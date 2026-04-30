use pyo3::prelude::*;
use pyo3::types::PyModule;

mod runtime;
mod settings;
mod task;

#[pymodule]
fn genja_core(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    runtime::register(module)?;
    settings::register(module)?;
    task::register(module)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    fn init_python() {
        static INIT: Once = Once::new();
        INIT.call_once(pyo3::prepare_freethreaded_python);
    }

    #[test]
    fn genja_core_module_registers_public_classes() {
        init_python();
        Python::with_gil(|py| {
            let module =
                PyModule::new(py, "test_genja_core_module").expect("test module should be created");

            genja_core(py, &module).expect("module initialization should succeed");

            assert!(module.getattr("Genja").is_ok());
            assert!(module.getattr("Settings").is_ok());
            assert!(module.getattr("CoreConfig").is_ok());
            assert!(module.getattr("TaskDefinition").is_ok());
            assert!(module.getattr("TaskResults").is_ok());
            assert!(module.getattr("HostTaskResult").is_ok());
        });
    }
}
