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
